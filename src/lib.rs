use std::net::{Ipv4Addr, SocketAddr};
use tokio::sync::{Mutex, Semaphore};
use std::sync::Arc;  
use std::time::Duration;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::collections::hash_map::DefaultHasher;
use tokio::net::{TcpListener, TcpStream}; 
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use chrono::Local; 

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
    service_discovery: Arc<ServiceDiscovery>,
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
            service_discovery: Arc::new(ServiceDiscovery::new()),
        }
    }

    pub async fn run(&self) -> Result<(), Box<dyn std::error::Error>> {
        let mut listener_tasks = Vec::new();
        let semaphore = Arc::new(Semaphore::new(self.max_concurrent));
        let (shutdown_tx, _shutdown_rx) = tokio::sync::broadcast::channel::<()>(1);

        for addr_data in self.addr_data.iter() {
            let permit = semaphore.clone().acquire_owned().await?;
            let error_registry = self.error_registry.clone();
            let socket_addr = socket_addr_create(addr_data.address, addr_data.port);
            let discovery = self.service_discovery.clone();
            let mut shutdown_rx = shutdown_tx.subscribe();
            
            let task = tokio::spawn(async move {
                match TcpListener::bind(&socket_addr).await {
                    Ok(listener) => {
                        println!("Listening on: {}", socket_addr);
                        
                        loop {
                            tokio::select! {
                                accept_result = tokio::time::timeout(
                                    Duration::from_secs(120), 
                                    listener.accept()
                                ) => {
                                    match accept_result {
                                        Ok(Ok((socket, addr))) => {
                                            let discovery = discovery.clone();
                                            tokio::spawn(async move {
                                                handle_connection(socket, addr, discovery).await;
                                            });
                                        }
                                        Ok(Err(e)) => {
                                            eprintln!("Accept error: {}", e);
                                        }
                                        Err(_) => {
                                            eprintln!("Connection timed out");
                                        }
                                    }
                                }
                                _ = shutdown_rx.recv() => {
                                    println!("Shutting down listener for {}", socket_addr);
                                    break;
                                }
                            }
                        }
                    }
                    Err(e) => {
                        let mut registry = error_registry.lock().await;
                        let error_id = registry.register_error(&e.to_string());
                        eprintln!("Bind error on {}: ID {}: {}", socket_addr, error_id, e);
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

async fn handle_connection(mut socket: TcpStream, addr: SocketAddr, discovery: Arc<ServiceDiscovery>) {
    // Try to detect if peer is serving content
    let mut detection_buf = [0u8; 1024];
    let mut content = String::new();
    
    // Send HTTP GET request
    let request = "GET / HTTP/1.1\r\nHost: localhost\r\n\r\n";
    if socket.write_all(request.as_bytes()).await.is_ok() {
        if let Ok(n) = socket.read(&mut detection_buf).await {
            if n > 0 {
                content = String::from_utf8_lossy(&detection_buf[..n]).to_string();
                discovery.record_service(addr, &content).await;
            }
        }
    }

    // Now serve our own content
    let response = format!(
        "HTTP/1.1 200 OK\r\n\
         Content-Type: text/html\r\n\
         \r\n\
         <html><body>\
         <h1>Port {}</h1>\
         <p>Active since: {}</p>\
         </body></html>",
        addr.port(),
        chrono::Local::now().format("%Y-%m-%d %H:%M:%S")
    );

    let _ = socket.write_all(response.as_bytes()).await;
}

// Helper functions
pub fn socket_addr_create(address: (u8, u8, u8, u8), port: u16) -> SocketAddr {
    SocketAddr::from((
        Ipv4Addr::new(address.0, address.1, address.2, address.3),
        port,
    ))
}

#[derive(Debug)]
pub struct ServiceDiscovery {
    log_file: PathBuf,
    discoveries: Arc<Mutex<HashMap<SocketAddr, String>>>,
}

impl ServiceDiscovery {
    pub fn new() -> Self {
        Self {
            log_file: PathBuf::from("discovered_services.txt"),
            discoveries: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn record_service(&self, addr: SocketAddr, content: &str) {
        let mut discoveries = self.discoveries.lock().await;
        discoveries.insert(addr, content.to_string());
        
        // Write to file
        if let Ok(mut file) = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.log_file) 
        {
            let _ = writeln!(file, "{}: {}", addr, content);
        }
    }
}