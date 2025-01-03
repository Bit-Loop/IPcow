use std::net::{Ipv4Addr, SocketAddr};
use tokio::sync::{Mutex, Semaphore};
use std::sync::Arc;  
use std::time::Duration;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::collections::hash_map::DefaultHasher;
use tokio::net::{TcpListener, TcpStream}; 
use tokio::io::{AsyncReadExt, AsyncWriteExt};

// Export public types and functions
pub mod sockparse;  // Move sockparse module here
pub use sockparse::addr_input;

#[derive(Debug, PartialEq, Clone)]
pub enum AddrType {
    IPv4,
    IPv6,
    TCP,
    UDP,
}

#[derive(Debug, Clone)]
pub struct AddrData {
    pub info: AddrType,
    pub socket_type: AddrType,
    pub address: (u8, u8, u8, u8),
    pub port: u16,
}

// Move ErrorRegistry and ListenerManager here
#[derive(Debug, Default)]
pub struct ErrorRegistry {
    errors: std::collections::HashMap<u64, String>
}

pub struct ListenerManager {
    error_registry: Arc<Mutex<ErrorRegistry>>,
    addr_data: Arc<Vec<AddrData>>,
    max_concurrent: usize,
    timeout_duration: Duration,
    max_retries: usize,
    retry_delay: Duration,
}

impl ErrorRegistry {
    pub fn new() -> Self {
        Self { errors: HashMap::new() }
    }

    pub fn register_error(&mut self, error: &str) -> u64 {
        let mut hasher = DefaultHasher::new();
        error.hash(&mut hasher);
        let id = hasher.finish();
        self.errors.entry(id).or_insert_with(|| error.to_string());
        id
    }
}

impl ListenerManager {
    pub fn new(addr_data: Vec<AddrData>, max_concurrent: usize) -> Self {
        Self {
            error_registry: Arc::new(Mutex::new(ErrorRegistry::new())),
            addr_data: Arc::new(addr_data),
            max_concurrent,
            timeout_duration: Duration::from_millis(100), // Reduced from 30 secs
            max_retries: 2,                               // Reduced from 3
            retry_delay: Duration::from_millis(10),       // Reduced from 1 sec
        }
    }

    pub async fn run(&self) -> Result<(), Box<dyn std::error::Error>> {
        let mut listener_tasks = Vec::new();
        let semaphore = Arc::new(Semaphore::new(self.max_concurrent));
        let (shutdown_tx, _) = tokio::sync::broadcast::channel::<()>(1);

        for addr_data in self.addr_data.iter() {
            let permit = semaphore.clone().acquire_owned().await?;
            let error_registry = self.error_registry.clone();
            let socket_addr = socket_addr_create(addr_data.address, addr_data.port);
            let timeout_duration = self.timeout_duration;
            let max_retries = self.max_retries;
            let retry_delay = self.retry_delay;
            let mut shutdown_rx = shutdown_tx.subscribe();
            
            let task = tokio::spawn(async move {
                let mut retry_count = 0;
                
                'connection: loop {
                    match TcpListener::bind(&socket_addr).await {
                        Ok(listener) => {
                            println!("Listening on: {}", socket_addr);
                            
                            loop {
                                tokio::select! {
                                    accept_result = tokio::time::timeout(timeout_duration, listener.accept()) => {
                                        match accept_result {
                                            Ok(Ok((socket, addr))) => {
                                                retry_count = 0; // Reset on successful connection
                                                tokio::spawn(async move {
                                                    if let Err(e) = socket.set_nodelay(true) {
                                                        eprintln!("Failed to set TCP_NODELAY: {}", e);
                                                    }
                                                    handle_connection(socket, addr).await;
                                                });
                                            }
                                            Ok(Err(e)) => {
                                                let mut registry = error_registry.lock().await;
                                                let error_id = registry.register_error(&e.to_string());
                                                eprintln!("Accept error on {}: ID {}: {}", socket_addr, error_id, e);
                                                
                                                if retry_count >= max_retries {
                                                    eprintln!("Max retries reached for {}", socket_addr);
                                                    break 'connection;
                                                }
                                                retry_count += 1;
                                                tokio::time::sleep(retry_delay).await;
                                            }
                                            Err(_) => {
                                                let mut registry = error_registry.lock().await;
                                                let error_id = registry.register_error("Connection timeout");
                                                eprintln!("Timeout on {}: ID {}", socket_addr, error_id);
                                                
                                                if retry_count >= max_retries {
                                                    eprintln!("Max retries reached for {}", socket_addr);
                                                    break 'connection;
                                                }
                                                retry_count += 1;
                                                tokio::time::sleep(retry_delay).await;
                                            }
                                        }
                                    }
                                    _ = shutdown_rx.recv() => {
                                        println!("Shutdown signal received for {}", socket_addr);
                                        break 'connection;
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            let mut registry = error_registry.lock().await;
                            let error_id = registry.register_error(&e.to_string());
                            eprintln!("Bind error on {}: ID {}: {}", socket_addr, error_id, e);
                            
                            if retry_count >= max_retries {
                                eprintln!("Max retries reached for {}", socket_addr);
                                break;
                            }
                            retry_count += 1;
                            tokio::time::sleep(retry_delay).await;
                        }
                    }
                }
                drop(permit);
            });
            
            listener_tasks.push(task);
        }

        futures::future::join_all(listener_tasks).await;
        Ok(())
    }
}

async fn handle_connection(mut socket: TcpStream, addr: SocketAddr) {
    if let Err(e) = socket.set_nodelay(true) {
        eprintln!("Failed to set TCP_NODELAY: {}", e);
    }
    let mut stream_buffer = [0u8; 8192];  // Increased buffer size
    
    while let Ok(n) = socket.read(&mut stream_buffer).await {
        if n == 0 || stream_buffer[..n].ends_with(&[13, 10]) {
            break;
        }
        if let Err(e) = socket.write_all(&stream_buffer[..n]).await {
            eprintln!("Failed to write to socket: {}", e);
            break;
        }
    }
}

// Helper functions
pub fn socket_addr_create(address: (u8, u8, u8, u8), port: u16) -> SocketAddr {
    SocketAddr::from((
        Ipv4Addr::new(address.0, address.1, address.2, address.3),
        port,
    ))
}