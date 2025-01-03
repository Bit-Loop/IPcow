use std::path::PathBuf;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::Mutex;
use std::fs::OpenOptions;
use std::io::Write;

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