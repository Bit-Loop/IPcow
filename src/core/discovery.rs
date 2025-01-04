use std::path::PathBuf;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::Mutex;
use std::fs::OpenOptions;
use std::io::Write;


/// ServiceDiscovery struct handles detection and logging of network services
/// Maintains thread-safe state of discovered services and their details
#[derive(Debug)]
pub struct ServiceDiscovery {
    // Path to log file where service discoveries are persisted
    log_file: PathBuf,
    // Thread-safe HashMap storing service details mapped to socket addresses
    discoveries: Arc<Mutex<HashMap<SocketAddr, String>>>,
}

impl ServiceDiscovery {
    /// Creates new ServiceDiscovery instance with default log file
    /// Initializes empty discoveries map protected by mutex
    pub fn new() -> Self {
        Self {
            log_file: PathBuf::from("discovered_services.txt"),
            discoveries: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Records discovered service information and logs it to file
    /// Args:
    ///   addr: Socket address where service was discovered
    ///   content: Service details/banner information
    pub async fn record_service(&self, addr: SocketAddr, content: &str) {
        // Update in-memory map of discoveries
        let mut discoveries = self.discoveries.lock().await;
        discoveries.insert(addr, content.to_string());
        
        // Append discovery to log file with timestamp and formatting
        if let Ok(mut file) = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.log_file) 
        {
            let timestamp = chrono::Local::now();
            // Format log entry with timestamp, address and content
            let formatted_entry = format!(
                "[{}] {}:{}\n{}\n{}\n", 
                timestamp,
                addr.ip(),  // Log IP address
                addr.port(), // Log port number
                "-".repeat(50), // Visual separator
                content.trim() // Actual service content
            );
            let _ = writeln!(file, "{}", formatted_entry);
        }
    }
}