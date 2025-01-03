use std::sync::Arc;
use std::net::SocketAddr;
use tokio::net::TcpStream;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use chrono::Local;
use crate::core::discovery::ServiceDiscovery;

pub async fn handle_connection(mut socket: TcpStream, addr: SocketAddr, discovery: Arc<ServiceDiscovery>) {
    let mut detection_buf = [0b; 1024];
    let mut content = String::new();
    
    let request = "GET / HTTP/1.1\r\nHost: localhost\r\n\r\n";
    if socket.write_all(request.as_bytes()).await.is_ok() {
        if let Ok(n) = socket.read(&mut detection_buf).await {
            if n > 0 {
                content = String::from_utf8_lossy(&detection_buf[..n]).to_string();
                discovery.record_service(addr, &content).await;
            }
        }
    }

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

    let _ = socket.write_all(response.as_bytes()).await;
}