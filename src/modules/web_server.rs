use serde_json;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::time::sleep;
use warp::Filter;

pub struct WebServer {
    port: u16,
}

impl WebServer {
    pub fn new() -> Self {
        Self { port: 3030 }
    }

    pub async fn start(&self) -> Result<(), Box<dyn std::error::Error>> {
        let routes = warp::path::end().map(|| "IPCow Web Interface");

        println!("Starting web server on port {}", self.port);
        warp::serve(routes).run(([127, 0, 0, 1], self.port)).await;

        Ok(())
    }
}

pub async fn run_web_server() {
    let server = WebServer::new();
    let _ = server.start().await;
}
