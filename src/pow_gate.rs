use std::{sync::{atomic::AtomicU8, Arc}, time::Duration};
use axum::body::Body;
use axum::extract::State;
use axum::http::{Request, Response, StatusCode};
use axum::middleware::Next;
use hmac::{Hmac, Mac};
use sha2::Sha256;
use moka::sync::Cache;

type HmacSha256 = Hmac<Sha256>;

pub struct PowState {
    pub secret: [u8; 32],
    pub difficulty: AtomicU8,
    pub seen: Cache<[u8; 16], ()>,
    pub window_secs: u64
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

    Ok(next.run(req).await)
}