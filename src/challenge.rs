use axum::body::Body;
use axum::extract::State;
use axum::http::{HeaderMap, Request, Response, StatusCode, header};
use axum::middleware::Next;
use hmac::{Hmac, KeyInit, Mac};
use moka::sync::Cache;
use rand::Rng;
use sha2::Sha256;
use std::sync::atomic::Ordering;
use std::sync::{Arc, atomic::AtomicU8};
use std::time::{SystemTime, UNIX_EPOCH};

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

fn create_mac(secret: [u8; 32], timestamp: u64, nonce: &[u8; 16], difficulty: u8) -> Hmac<Sha256> {
    let mut m: Hmac<Sha256> = Hmac::new_from_slice(&secret).unwrap();
    m.update(&timestamp.to_be_bytes());
    m.update(nonce);
    m.update(&[difficulty]);
    m
}

// X-Challenge: timestamp.nonce.difficulty.mac.counter
// (client appends counter)
pub async fn check_challenge(
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

    let m = create_mac(st.secret, timestamp, &nonce, difficulty);
    m.verify_slice(&mac).map_err(|_| StatusCode::FORBIDDEN)?;

    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
    if now.abs_diff(timestamp) > st.window_secs || difficulty < st.difficulty.load(Ordering::Relaxed) {
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

// X-Challenge: timestamp.nonce.difficulty.mac
pub async fn mint_challenge(State(st): State<Arc<PowState>>) -> HeaderMap {
    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
    let mut nonce = [0u8; 16];
    rand::rng().fill_bytes(&mut nonce);
    let diff = st.difficulty.load(Ordering::Relaxed);
    let mac = hex::encode(create_mac(st.secret, timestamp, &nonce, diff).finalize().into_bytes());

    let challenge = format!("{}.{}.{}.{}", timestamp, hex::encode(nonce), diff, mac);

    let mut headers = HeaderMap::new();
    headers.insert("x-challenge", challenge.parse().unwrap());
    headers.insert(header::CACHE_CONTROL, "no-store".parse().unwrap());
    headers
}