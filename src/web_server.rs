use warp::Filter;
use std::time::Duration;
use tokio::time::sleep;
use std::sync::Arc;
use tokio::sync::Mutex;

// Add shared state for metrics
#[derive(Clone)]
struct ServerState {
    connections: Arc<Mutex<usize>>,
    uptime: Arc<Mutex<Duration>>,
}

pub async fn run_web_server() {
    let state = ServerState {
        connections: Arc::new(Mutex::new(0)),
        uptime: Arc::new(Mutex::new(Duration::from_secs(0))),
    };

    // Status endpoint
    let state_clone = state.clone();
    let status = warp::path("status")
        .map(move || {
            let conns = state_clone.connections.try_lock().unwrap_or(&mut 0);
            let uptime = state_clone.uptime.try_lock().unwrap_or(&mut Duration::from_secs(0));
            warp::reply::json(&serde_json::json!({
                "status": "running",
                "connections": *conns,
                "uptime_secs": uptime.as_secs()
            }))
        });

    // Health check endpoint
    let health = warp::path("health")
        .map(|| warp::reply::with_status("OK", warp::http::StatusCode::OK));

    // Combine routes
    let routes = warp::get().and(
        status
            .or(health)
            .or(warp::path::end().map(|| warp::reply::html("IPcow Web Interface")))
    );

    // Start server with shutdown
    let (addr, server) = warp::serve(routes)
        .bind_with_graceful_shutdown(([127, 0, 0, 1], 3030), async {
            sleep(Duration::from_secs(120)).await;
            println!("Web server shutting down...");
        });

    println!("Web server running on http://{}", addr);
    server.await;
}