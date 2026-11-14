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

pub async fn pow_gate(
    State(st): State<Arc<PowState>>,
    req: Request<Body>,
    next: Next
) -> Result<Response<Body>, StatusCode> {
    Ok(next.run(req).await)
}