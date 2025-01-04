// Network connection handler module implementing connection processing and service detection

use std::sync::Arc;
use std::net::SocketAddr;
use tokio::net::TcpStream;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use chrono::Local;
use crate::core::discovery::ServiceDiscovery;

/// Main connection handler function that processes new TCP connections
/// Performs service detection and responds with connection status
/// Args:
///   socket: Active TCP connection
///   addr: Remote peer address
///   discovery: Shared service detection system
pub async fn handle_connection(mut socket: TcpStream, addr: SocketAddr, discovery: Arc<ServiceDiscovery>) {
    // Buffer for reading service detection data
    let mut detection_buf = [0_u8; 1024];
    let mut content = String::new();
    
    // Send HTTP request to probe for service information
    let request = "GET / HTTP/1.1\r\nHost: localhost\r\n\r\n";
    if socket.write_all(request.as_bytes()).await.is_ok() {
        // Read response for service fingerprinting
        if let Ok(n) = socket.read(&mut detection_buf).await {
            if n > 0 {
                // Convert response to string and record service details
                content = String::from_utf8_lossy(&detection_buf[..n]).to_string();
                discovery.record_service(addr, &content).await;
            }
        }
    }

    // Prepare and send HTTP response with connection details
    // Includes port number and connection timestamp
    let response = format!(
        "HTTP/1.1 200 OK\r\n\
         Content-Type: text/html\r\n\
         \r\n\
         <html><body>\
         <h1>Port {}</h1>\
         <p>Active since: {}</p>\
         </body></html>",
        addr.port(),
        Local::now().format("%Y-%m-%d %H:%M:%S")
    );

    // Send response back to client
    let _ = socket.write_all(response.as_bytes()).await;
}