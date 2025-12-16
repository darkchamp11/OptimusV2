mod handlers;
mod routes;

use axum::Router;
use redis::aio::ConnectionManager;
use std::sync::Arc;
use tokio::net::TcpListener;

#[derive(Clone)]
pub struct AppState {
    pub redis: ConnectionManager,
}

#[tokio::main]
async fn main() {
    println!("Optimus API booting...");

    // Connect to Redis
    let redis_url = std::env::var("REDIS_URL")
        .unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string());
    
    let client = redis::Client::open(redis_url.as_str())
        .expect("Failed to create Redis client");
    
    let redis_conn = ConnectionManager::new(client).await
        .expect("Failed to connect to Redis");
    
    println!("✓ Connected to Redis: {}", redis_url);

    let state = Arc::new(AppState {
        redis: redis_conn,
    });

    // Build router
    let app = Router::new()
        .merge(routes::routes())
        .with_state(state);

    // Start server
    let addr = "0.0.0.0:3000";
    let listener = TcpListener::bind(addr).await
        .expect("Failed to bind to address");
    
    println!("✓ HTTP server listening on {}", addr);
    println!("Ready to accept jobs");

    axum::serve(listener, app).await
        .expect("Server error");
}
