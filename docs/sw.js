// GitMine TTY — Service Worker (offline-first PWA cache)
const CACHE = 'gitmine-tty-v1';
const STATIC = [
  './',
  './index.html',
  './manifest.json',
  './fennec/client.js',
  'https://cdn.jsdelivr.net/npm/chart.js@4.4.3/dist/chart.umd.min.js',
];
const DYNAMIC_STALE = [
  './template.json',
  './stats.json',
  './payouts.json',
  './config.json',
];

self.addEventListener('install', e => {
  e.waitUntil(caches.open(CACHE).then(c => c.addAll(STATIC)));
  self.skipWaiting();
});

self.addEventListener('activate', e => {
  e.waitUntil(caches.keys().then(keys =>
    Promise.all(keys.filter(k => k !== CACHE).map(k => caches.delete(k)))
  ));
  self.clients.claim();
});

self.addEventListener('fetch', e => {
  const url = new URL(e.request.url);
  const isDynamic = DYNAMIC_STALE.some(p => url.pathname.endsWith(p.replace('./', '')));

  if (isDynamic) {
    // Stale-while-revalidate for pool state files
    e.respondWith(
      caches.open(CACHE).then(async cache => {
        const cached = await cache.match(e.request);
        const fresh = fetch(e.request).then(res => {
          if (res.ok) cache.put(e.request, res.clone());
          return res;
        }).catch(() => null);
        return cached || fresh;
      })
    );
  } else {
    // Cache-first for static assets
    e.respondWith(
      caches.match(e.request).then(r => r || fetch(e.request))
    );
  }
});
