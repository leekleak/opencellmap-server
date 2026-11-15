use axum::body::Body;
use axum::extract::State;
use axum::http::{Request, Response, StatusCode};
use axum::middleware::Next;
use hmac::{Hmac, KeyInit, Mac};
use moka::sync::Cache;
use sha2::Sha256;
use std::sync::atomic::Ordering;
use std::time::{SystemTime, UNIX_EPOCH};
use std::sync::{Arc, atomic::AtomicU8};

pub struct PowState {
    pub secret: [u8; 32],
    pub difficulty: AtomicU8,
    pub seen: Cache<[u8; 16], ()>,
    pub window_secs: u64
}

fn leading_zero_bits(hash: &[u8; 32]) -> u32 {
    let mut bits = 0;
    for byte in hash {
        if *byte == 0 {
            bits += 8;
        } else {
            bits += byte.leading_zeros();
            break;
        }
    }
    bits
}

fn get_hash(nonce: &[u8; 16], timestamp: u64, counter: u64) -> [u8; 32] {
    let mut hash = blake3::Hasher::new();
    hash.update(nonce);
    hash.update(&timestamp.to_be_bytes());
    hash.update(&counter.to_be_bytes());
    *hash.finalize().as_bytes()
}

//X-Challenge: timestamp.nonce.difficulty.mac.counter
pub async fn pow_gate(
    State(st): State<Arc<PowState>>,
    req: Request<Body>,
    next: Next
) -> Result<Response<Body>, StatusCode> {
    let header = req.headers().get("x-challenge")
        .and_then(|header| header.to_str().ok()).ok_or(StatusCode::FORBIDDEN)?;
    let parts: Vec<&str> = header.split(".").collect();
    if parts.len() != 5 {return Err(StatusCode::BAD_REQUEST)}

    let timestamp: u64 = parts[0].parse().map_err(|_| StatusCode::BAD_REQUEST)?;
    let nonce: [u8; 16] = hex::decode(parts[1]).ok().and_then(|v| v.try_into().ok())
        .ok_or(StatusCode::BAD_REQUEST)?;
    let difficulty: u8 = parts[2].parse().map_err(|_| StatusCode::BAD_REQUEST)?;
    let mac = hex::decode(parts[3]).map_err(|_| StatusCode::BAD_REQUEST)?;
    let counter: u64 = parts[4].parse().map_err(|_| StatusCode::BAD_REQUEST)?;

    let mut m: Hmac<Sha256> = Hmac::new_from_slice(&st.secret).unwrap();
    m.update(&timestamp.to_be_bytes());
    m.update(&nonce);
    m.update(&[difficulty]);
    m.verify_slice(&mac).map_err(|_| StatusCode::FORBIDDEN)?;

    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
    if now.saturating_sub(timestamp) > st.window_secs || difficulty < st.difficulty.load(Ordering::Relaxed) {
        return Err(StatusCode::FORBIDDEN);
    }

    if leading_zero_bits(&get_hash(&nonce, timestamp, counter)) < difficulty as u32 {
        return Err(StatusCode::FORBIDDEN);
    }

    if !st.seen.entry(nonce).or_insert(()).is_fresh() {
        return Err(StatusCode::FORBIDDEN);
    }

    Ok(next.run(req).await)
}