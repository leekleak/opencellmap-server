mod challenge;

use crate::challenge::{check_challenge, PowState, mint_challenge};
use axum::{Router, middleware, routing::get, routing::post};
use moka::sync::Cache;
use rand::Rng;
use std::sync::Arc;
use std::sync::atomic::AtomicU8;
use std::time::Duration;

#[tokio::main]
async fn main() {
    let window_secs = 10;

    let seen: Cache<[u8; 16], ()> = Cache::builder()
        .time_to_live(Duration::from_secs(window_secs + 5))
        .max_capacity(1_000_000)
        .build();

    let mut secret = [0u8; 32];
    rand::rng().fill_bytes(&mut secret);

    let state = Arc::new(
        PowState {
            secret,
            difficulty: AtomicU8::new(20),
            seen,
            window_secs
        }
    );

    // build our application with a single route
    let app = Router::new()
        .route("/id/challenge/get", get(mint_challenge))
        .route("/post", post(post_json).route_layer(middleware::from_fn_with_state(state.clone(), check_challenge)))
        .with_state(state);

    // run our app with hyper, listening globally on port 3000
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn post_json() -> String {
    String::from("Hello, World!")
}
