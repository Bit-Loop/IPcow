use tokio::net::TcpListener;
use tokio::io::{AsyncReadExt};
use std::time::Duration;
use std::sync::Arc;
use tokio::sync::Mutex;

const TEST_PORT_1: u16 = 9999;
const TEST_PORT_2: u16 = 9998;
const TEST_DURATION: u64 = 10;

#[tokio::test]
async fn test_network_throughput() {
    let addr1 = format!("127.0.0.1:{}", TEST_PORT_1);
    let addr2 = format!("127.0.0.1:{}", TEST_PORT_2);
    
    let listener1 = TcpListener::bind(&addr1).await.expect("Failed to bind to first port");
    let listener2 = TcpListener::bind(&addr2).await.expect("Failed to bind to second port");
    
    let total_bytes = Arc::new(Mutex::new(0));
    let start_time = std::time::Instant::now();

    // Test connection handling
    let _handle1 = spawn_listener(listener1, Arc::clone(&total_bytes));
    let _handle2 = spawn_listener(listener2, Arc::clone(&total_bytes));

    tokio::time::sleep(Duration::from_secs(TEST_DURATION)).await;

    let elapsed = start_time.elapsed().as_secs_f64();
    let total_bytes = total_bytes.lock().await;
    let bitrate = (*total_bytes as f64 * 8.0) / elapsed;

    assert!(bitrate > 0.0, "Bitrate should be greater than 0");
}

async fn spawn_listener(listener: TcpListener, total_bytes: Arc<Mutex<usize>>) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        loop {
            match listener.accept().await {
                Ok((mut socket, _)) => {
                    let total_bytes = Arc::clone(&total_bytes);
                    tokio::spawn(async move {
                        handle_connection(&mut socket, total_bytes).await;
                    });
                }
                Err(_) => break,
            }
        }
    })
}

async fn handle_connection(socket: &mut tokio::net::TcpStream, total_bytes: Arc<Mutex<usize>>) {
    let mut buf = [0; 1024];
    loop {
        match socket.read(&mut buf).await {
            Ok(0) => break,
            Ok(n) => {
                let mut counter = total_bytes.lock().await;
                *counter += n;
            }
            Err(_) => break,
        }
    }
}