-- GitMine Pool — Supabase Schema
-- Run this in your Supabase SQL editor (Database → SQL Editor → New query)
-- Safe to re-run (all DDL uses IF NOT EXISTS / OR REPLACE).

-- ─── Users ────────────────────────────────────────────────────────────────────
-- Mirrors GitHub identity. Created/upserted on first dashboard login.
create table if not exists users (
  id           uuid primary key default gen_random_uuid(),
  github_id    bigint unique not null,
  github_login text unique not null,   -- e.g. "alice"
  avatar_url   text,
  created_at   timestamptz default now(),
  last_seen_at timestamptz default now()
);

-- ─── Workers ──────────────────────────────────────────────────────────────────
-- One row per username.suffix the miner uses as their Stratum username.
-- worker_name must contain at least one '.' (e.g. "alice.rig1").
create table if not exists workers (
  id          uuid primary key default gen_random_uuid(),
  user_id     uuid references users(id) on delete cascade,  -- nullable until github-login links it
  worker_name text unique not null,   -- e.g. "alice.gitmine", "alice.asic01"
  coin        text not null default 'FNNC', -- 'FNNC' | 'TTY'
  enabled     boolean not null default true,
  created_at  timestamptz default now(),
  constraint worker_name_prefix check (worker_name like '%' || '.' || '%')
);

create index if not exists workers_user_id_idx on workers(user_id);
create index if not exists workers_name_idx    on workers(worker_name);

-- ─── Shares ───────────────────────────────────────────────────────────────────
create table if not exists shares (
  id           bigserial primary key,
  worker_id    uuid references workers(id) on delete cascade,  -- nullable for fast inserts
  worker_name  text not null,   -- denormalized; resolves the join from the dashboard
  coin         text not null,   -- 'FNNC' | 'TTY'
  algo         text not null,   -- 'yescryptr16' | 'sha256d'
  difficulty   numeric not null default 1,
  accepted     boolean not null default true,
  block_height bigint,
  submitted_at timestamptz default now()
);

create index if not exists shares_worker_id_idx   on shares(worker_id);
create index if not exists shares_worker_name_idx on shares(worker_name);  -- fast per-worker queries
create index if not exists shares_submitted_idx   on shares(submitted_at desc);
create index if not exists shares_coin_idx        on shares(coin);

-- ─── Balances ─────────────────────────────────────────────────────────────────
create table if not exists balances (
  id       uuid primary key default gen_random_uuid(),
  user_id  uuid not null references users(id) on delete cascade,
  coin     text not null,
  amount   numeric(20, 8) not null default 0,
  unique(user_id, coin)
);

-- ─── Payouts ──────────────────────────────────────────────────────────────────
create table if not exists payouts (
  id         uuid primary key default gen_random_uuid(),
  user_id    uuid not null references users(id) on delete cascade,
  coin       text not null,
  amount     numeric(20, 8) not null,
  address    text not null,
  txid       text,
  status     text not null default 'pending',  -- 'pending' | 'broadcast' | 'confirmed'
  created_at timestamptz default now(),
  updated_at timestamptz default now()
);

-- ─── Pool stats (snapshot, updated by Actions cron every 2 min) ───────────────
create table if not exists pool_stats (
  id             bigserial primary key,
  coin           text not null,          -- 'FNNC' | 'TTY'
  total_shares   bigint not null default 0,
  hashrate_mhs   numeric(16,4) not null default 0,
  active_workers int not null default 0,
  block_height   bigint,
  template_algo  text,
  snapshot_at    timestamptz default now()
);

-- ─── Realtime Publications ────────────────────────────────────────────────────
-- Enables Supabase Realtime (postgres_changes) on shares so the dashboard
-- receives live flashes for each accepted share.
alter publication supabase_realtime add table shares;
alter publication supabase_realtime add table pool_stats;

-- ─── Row Level Security ───────────────────────────────────────────────────────
alter table users      enable row level security;
alter table workers    enable row level security;
alter table shares     enable row level security;
alter table balances   enable row level security;
alter table payouts    enable row level security;
alter table pool_stats enable row level security;

-- pool_stats: public read (anyone can see pool overview)
drop policy if exists "pool_stats public read" on pool_stats;
create policy "pool_stats public read"
  on pool_stats for select using (true);

-- workers: PUBLIC READ so the All-Miners table can list everyone.
-- Inserts/updates are own-row only (authenticated user owning the worker).
drop policy if exists "workers public read" on workers;
create policy "workers public read"
  on workers for select using (true);

drop policy if exists "workers own insert" on workers;
create policy "workers own insert" on workers for insert with check (
  user_id is null  -- anonymous pool-side insert before linking
  or user_id::text = auth.uid()::text
);

drop policy if exists "workers own update" on workers;
create policy "workers own update" on workers for update using (
  user_id::text = auth.uid()::text
);

drop policy if exists "workers own delete" on workers;
create policy "workers own delete" on workers for delete using (
  user_id::text = auth.uid()::text
);

-- users: own row only
drop policy if exists "users own row read"   on users;
drop policy if exists "users own row update" on users;
create policy "users own row read"   on users for select using (auth.uid()::text = id::text);
create policy "users own row update" on users for update using (auth.uid()::text = id::text);

-- shares: public read (pool dashboard lists all; miners can only see their own via app)
drop policy if exists "shares public read" on shares;
create policy "shares public read"
  on shares for select using (true);

-- shares insert: pool server writes via service_role key (bypasses RLS)
-- No anon/user insert policy needed.

-- balances: own rows
drop policy if exists "balances own read" on balances;
create policy "balances own read" on balances for select using (
  user_id::text = auth.uid()::text
);

-- payouts: own rows
drop policy if exists "payouts own read" on payouts;
create policy "payouts own read" on payouts for select using (
  user_id::text = auth.uid()::text
);

-- ─── Worker resolution helper ─────────────────────────────────────────────────
-- Called by relay/worker_resolver.py with the Stratum username.
-- Extracts the GitHub login prefix, looks up the user, auto-creates the worker
-- if it doesn't exist yet.  Returns nothing if the prefix is unknown (reject).
-- Runs SECURITY DEFINER so it bypasses RLS; pool calls it with service_role key.
create or replace function resolve_worker(
  p_worker_name  text,
  p_github_login text   default null,
  p_github_id    bigint default null,
  p_avatar_url   text   default null,
  p_coin         text   default 'FNNC'
)
returns table (
  user_id     uuid,
  worker_id   uuid,
  worker_name text,
  coin        text,
  enabled     boolean
)
language plpgsql security definer as $$
declare
  v_prefix    text;
  v_user_id   uuid;
  v_worker_id uuid;
begin
  v_prefix := split_part(p_worker_name, '.', 1);

  -- Upsert user if full GitHub info provided (relay has it from prior /resolve call)
  if p_github_login is not null and p_github_id is not null then
    insert into users (github_id, github_login, avatar_url, last_seen_at)
    values (p_github_id, p_github_login, p_avatar_url, now())
    on conflict (github_login) do update
      set last_seen_at = now(),
          avatar_url   = coalesce(excluded.avatar_url, users.avatar_url)
    returning id into v_user_id;
  else
    -- Relay only has the worker name; look up by login prefix
    select id into v_user_id from users where github_login = v_prefix;
  end if;

  -- Unknown prefix → return empty set → pool rejects the connection
  if v_user_id is null then
    return;
  end if;

  -- Upsert worker; update coin in case miner changed it
  insert into workers (user_id, worker_name, coin)
  values (v_user_id, p_worker_name, p_coin)
  on conflict (worker_name) do update
    set coin    = excluded.coin,
        user_id = coalesce(workers.user_id, excluded.user_id)
  returning id into v_worker_id;

  return query
    select v_user_id, v_worker_id, p_worker_name, p_coin, true;
end;
$$;

-- ─── insert_share helper ──────────────────────────────────────────────────────
-- Pool server calls this instead of a raw INSERT so it can pass worker_name
-- without knowing the worker UUID ahead of time.
create or replace function insert_share(
  p_worker_name  text,
  p_coin         text,
  p_algo         text,
  p_difficulty   numeric,
  p_accepted     boolean default true,
  p_block_height bigint  default null
)
returns bigint
language plpgsql security definer as $$
declare
  v_worker_id uuid;
  v_share_id  bigint;
begin
  select id into v_worker_id from workers where worker_name = p_worker_name;

  insert into shares (worker_id, worker_name, coin, algo, difficulty, accepted, block_height)
  values (v_worker_id, p_worker_name, p_coin, p_algo, p_difficulty, p_accepted, p_block_height)
  returning id into v_share_id;

  return v_share_id;
end;
$$;
