use axum::{routing::get, routing::post, Router};

#[tokio::main]
async fn main() {
    // build our application with a single route
    let app = Router::new()
        .route("/id/challenge/get", get(get_id_challenge))
        .route("/id/challenge/result", get(get_id_challenge_result).post(post_id_challenge_result))
        .route("/id/challenge/verify", get(get_id_challenge_verify))
        .route("/post", post(post_json));

    // run our app with hyper, listening globally on port 3000
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn get_id_challenge() -> String {
    String::from("Hello, World!")
}

async fn get_id_challenge_result() -> String {
    String::from("Hello, World!")
}

async fn post_id_challenge_result() -> String {
    String::from("Hello, World!")
}

async fn get_id_challenge_verify() -> String {
    String::from("Hello, World!")
}

async fn post_json() -> String {
    String::from("Hello, World!")
}
