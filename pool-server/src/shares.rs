use crate::jobs::{Job, sha256d, nbits_to_target};
use anyhow::{bail, Result};
use hex;

#[derive(Debug)]
pub struct ShareResult {
    pub accepted:    bool,
    pub is_block:    bool,
    pub difficulty:  f64,
    pub hash:        String,
}

/// Validate a submitted share against the active job.
/// Returns ShareResult on success, Err on malformed input.
pub fn validate(
    job:          &Job,
    extranonce1:  &str,
    extranonce2:  &str,
    ntime:        &str,
    nonce:        &str,
    client_diff:  f64,
) -> Result<ShareResult> {
    // Build coinbase = coinbase1 + extranonce1 + extranonce2 + coinbase2
    let coinbase_hex = format!("{}{}{}{}", job.coinbase1, extranonce1, extranonce2, job.coinbase2);
    let coinbase_bytes = hex::decode(&coinbase_hex)
        .map_err(|_| anyhow::anyhow!("bad coinbase hex"))?;
    let coinbase_hash = sha256d(&coinbase_bytes);

    // Compute merkle root by folding branches
    let mut merkle_root = coinbase_hash;
    for branch in &job.merkle_branches {
        let branch_bytes = hex::decode(branch)
            .map_err(|_| anyhow::anyhow!("bad merkle branch"))?;
        let mut combined = [0u8; 64];
        combined[..32].copy_from_slice(&merkle_root);
        combined[32..].copy_from_slice(&branch_bytes);
        merkle_root = sha256d(&combined);
    }

    // Build 80-byte block header
    let version_bytes = job.version.to_le_bytes();
    let prev_hash = decode_reversed(&job.prev_hash)?;
    let ntime_bytes = u32::from_str_radix(ntime, 16)
        .map_err(|_| anyhow::anyhow!("bad ntime"))?
        .to_le_bytes();
    let nbits_bytes = u32::from_str_radix(&job.nbits, 16)
        .map_err(|_| anyhow::anyhow!("bad nbits"))?
        .to_le_bytes();
    let nonce_bytes = u32::from_str_radix(nonce, 16)
        .map_err(|_| anyhow::anyhow!("bad nonce"))?
        .to_le_bytes();

    let mut header = [0u8; 80];
    header[0..4].copy_from_slice(&version_bytes);
    header[4..36].copy_from_slice(&prev_hash);
    header[36..68].copy_from_slice(&merkle_root);
    header[68..72].copy_from_slice(&ntime_bytes);
    header[72..76].copy_from_slice(&nbits_bytes);
    header[76..80].copy_from_slice(&nonce_bytes);

    // Hash the header
    let hash = match job.algo.as_str() {
        "sha256d" => sha256d(&header),
        // FNNC (yescryptr16) placeholder — returns a stub until we bind the C impl
        _ => sha256d(&header),
    };

    let hash_hex = hex::encode(hash);

    // Compare hash against client target (derived from client_diff)
    let client_target = diff_to_target(client_diff);
    if !hash_meets_target(&hash, &client_target) {
        bail!("share does not meet difficulty target");
    }

    // Check if this is a block solution (meets network target)
    let is_block = hash_meets_target(&hash, &job.target);

    Ok(ShareResult {
        accepted: true,
        is_block,
        difficulty: client_diff,
        hash: hash_hex,
    })
}

/// Decode a hex prev_hash and reverse each 4-byte chunk (Stratum convention).
fn decode_reversed(hex_str: &str) -> Result<[u8; 32]> {
    let raw = hex::decode(hex_str)
        .map_err(|_| anyhow::anyhow!("bad prev_hash hex"))?;
    if raw.len() != 32 { bail!("prev_hash must be 32 bytes"); }
    let mut out = [0u8; 32];
    for i in 0..8 {
        let chunk = &raw[i*4..(i+1)*4];
        out[i*4..i*4+4].copy_from_slice(&[chunk[3], chunk[2], chunk[1], chunk[0]]);
    }
    Ok(out)
}

/// Convert difficulty to 32-byte target.
fn diff_to_target(diff: f64) -> [u8; 32] {
    // Bitcoin difficulty 1 target = 0x00000000FFFF0000...0000
    // target = diff1_target / difficulty
    let diff1: f64 = 0x00000000ffff0000_u64 as f64 * (1u64 << 208) as f64;
    let target_val = diff1 / diff;
    // Encode as 32-byte big-endian
    let mut target = [0u8; 32];
    // Simple approximation: set the high bytes
    let hi = (target_val / (1u64 << 32) as f64) as u64;
    let lo = (target_val % (1u64 << 32) as f64) as u64;
    target[24..28].copy_from_slice(&(hi as u32).to_be_bytes());
    target[28..32].copy_from_slice(&(lo as u32).to_be_bytes());
    target
}

/// True if hash ≤ target (both 32-byte big-endian).
fn hash_meets_target(hash: &[u8; 32], target: &[u8; 32]) -> bool {
    for i in 0..32 {
        if hash[i] < target[i] { return true; }
        if hash[i] > target[i] { return false; }
    }
    true
}
