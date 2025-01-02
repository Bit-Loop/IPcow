// src/main_test.rs
#[cfg(test)]
mod tests {
    use sysinfo::{System, SystemExt, ProcessorExt};
    use tokio::net::TcpListener;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use std::time::Duration;
    use std::sync::Arc;
    use tokio::sync::Mutex;
    use chrono::Local;

    #[tokio::test]
    async fn test_system_monitoring() {
        // Initialize system info
        let mut system = System::new_all();
        system.refresh_all();

        // Print CPU and memory usage
        println!("Total memory: {} KB", system.total_memory());
        println!("Used memory : {} KB", system.used_memory());
        println!("Total swap  : {} KB", system.total_swap());
        println!("Used swap   : {} KB", system.used_swap());

        for processor in system.processors() {
            println!("{}: {}%", processor.name(), processor.cpu_usage());
        }

        // Set up TCP listeners
        let addr1 = "0.0.0.0:9999".to_string();
        let addr2 = "0.0.0.0:9998".to_string();
        let listener1 = TcpListener::bind(&addr1).await.unwrap();
        let listener2 = TcpListener::bind(&addr2).await.unwrap();
        println!("Listening on: {} and {}", addr1, addr2);

        // Shared state for tracking bitrate
        let total_bytes = Arc::new(Mutex::new(0));
        let start_time = std::time::Instant::now();

        // Accept connections and print bitrate
        let total_bytes_clone = Arc::clone(&total_bytes);
        tokio::spawn(async move {
            loop {
                let (mut socket, _) = listener1.accept().await.unwrap();
                let total_bytes_clone = Arc::clone(&total_bytes_clone);
                tokio::spawn(async move {
                    let mut buf = [0; 1024];
                    loop {
                        let n = socket.read(&mut buf).await.unwrap();
                        if n == 0 {
                            break;
                        }
                        let mut total_bytes = total_bytes_clone.lock().await;
                        *total_bytes += n;
                    }
                });
            }
        });

        let total_bytes_clone = Arc::clone(&total_bytes);
        tokio::spawn(async move {
            loop {
                let (mut socket, _) = listener2.accept().await.unwrap();
                let total_bytes_clone = Arc::clone(&total_bytes_clone);
                tokio::spawn(async move {
                    let mut buf = [0; 1024];
                    loop {
                        let n = socket.read(&mut buf).await.unwrap();
                        if n == 0 {
                            break;
                        }
                        let mut total_bytes = total_bytes_clone.lock().await;
                        *total_bytes += n;

                        // Send "Hello, World!" and current time
                        let response = format!("Hello, World! Current time: {}\n", Local::now().format("%Y-%m-%d %H:%M:%S"));
                        socket.write_all(response.as_bytes()).await.unwrap();
                    }
                });
            }
        });

        // Run the test for a limited duration
        tokio::time::sleep(Duration::from_secs(10)).await;

        // Calculate bitrate
        let elapsed = start_time.elapsed().as_secs_f64();
        let total_bytes = total_bytes.lock().await;
        let bitrate = (*total_bytes as f64 * 8.0) / elapsed;
        println!("Bitrate: {} bps", bitrate);

        // Assert that bitrate is greater than 0
        assert!(bitrate > 0.0, "Bitrate should be greater than 0");
    }
}