use std::net::{IpAddr, SocketAddr};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::net::TcpSocket;
use chrono::{DateTime, Local, NaiveDateTime};
use serde::{Serialize, Deserialize};
use crate::core::types::{NetworkResult, NetworkError};
use tokio::fs::OpenOptions;

const PING_TIMEOUT: Duration = Duration::from_millis(500);
const CONNECT_TIMEOUT: Duration = Duration::from_millis(200);
const LOG_FILE: &str = "host_status.log";

#[derive(Debug, Clone, Serialize, Deserialize)]
struct HostStatus {
    last_alive: DateTime<Local>,
    last_down: Option<DateTime<Local>>,
    current_state: HostState,
    #[serde(with = "duration_serde")]
    total_downtime: Duration,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
enum HostState {
    Alive,
    Dead,
}

// Helper module for serializing Duration
mod duration_serde {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::time::Duration;

    pub fn serialize<S>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        duration.as_secs().serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Duration, D::Error>
    where
        D: Deserializer<'de>,
    {
        let secs = u64::deserialize(deserializer)?;
        Ok(Duration::from_secs(secs))
    }
}

struct HostTracker {
    hosts: Arc<Mutex<HashMap<IpAddr, HostStatus>>>,
}

impl HostTracker {
    fn new() -> Self {
        Self {
            hosts: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    async fn update_host_status(&self, ip: IpAddr, is_alive: bool) {
        let mut hosts = self.hosts.lock().await;
        let now = Local::now();
        
        let status = hosts.entry(ip).or_insert(HostStatus {
            last_alive: now,
            last_down: None,
            current_state: HostState::Alive,
            total_downtime: Duration::from_secs(0),
        });

        match (is_alive, status.current_state) {
            (true, HostState::Dead) => {
                status.last_alive = now;
                status.current_state = HostState::Alive;
                if let Some(down_time) = status.last_down {
                    let downtime = now.signed_duration_since(down_time)
                        .to_std()
                        .unwrap_or_default();
                    status.total_downtime += downtime;
                }
                self.log_state_change(ip, "RECOVERED", status).await.unwrap();
            }
            (false, HostState::Alive) => {
                status.last_down = Some(now);
                status.current_state = HostState::Dead;
                self.log_state_change(ip, "DOWN", status).await.unwrap();
            }
            _ => {}
        }
    }

    async fn log_state_change(&self, ip: IpAddr, event: &str, status: &HostStatus) -> NetworkResult<()> {
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(LOG_FILE)
            .await
            .map_err(|e| NetworkError::IoError(e))?;

        let entry = format!(
            "[{}] {} {} | Last alive: {} | Last down: {} | Total downtime: {:.2}s\n",
            Local::now().format("%Y-%m-%d %H:%M:%S"),
            ip,
            event,
            status.last_alive.format("%Y-%m-%d %H:%M:%S"),
            status.last_down.map_or("N/A".to_string(), |t| t.format("%Y-%m-%d %H:%M:%S").to_string()),
            status.total_downtime.as_secs_f64()
        );

        use tokio::io::AsyncWriteExt;
        file.write_all(entry.as_bytes())
            .await
            .map_err(|e| NetworkError::IoError(e))?;

        Ok(())
    }

    async fn get_host_status(&self, ip: IpAddr) -> Option<HostStatus> {
        self.hosts.lock().await.get(&ip).cloned()
    }

    async fn print_status(&self, ip: IpAddr) {
        if let Some(status) = self.get_host_status(ip).await {
            println!("\nHost Status for {}:", ip);
            println!("Current State: {:?}", status.current_state);
            println!("Last Alive: {}", status.last_alive.format("%Y-%m-%d %H:%M:%S"));
            
            if let Some(down_time) = status.last_down {
                println!("Last Down: {}", down_time.format("%Y-%m-%d %H:%M:%S"));
                println!("Total Downtime: {:.2}s", status.total_downtime.as_secs_f64());
            }
            println!("------------------------");
        } else {
            println!("No history for host {}", ip);
        }
    }
}

/// Performs TCP SYN scan on target address
async fn syn_scan(addr: SocketAddr) -> NetworkResult<bool> {
    let socket = TcpSocket::new_v4()?;
    
    // Use non-blocking connect for SYN scanning
    match tokio::time::timeout(CONNECT_TIMEOUT, socket.connect(addr)).await {
        Ok(Ok(_)) => Ok(true),   // SYN-ACK received
        Ok(Err(_)) => Ok(false), // RST received
        Err(_) => Ok(false),     // Timeout - no response
    }
}

/// Ping a range of ports on target IPs using SYN scanning
pub async fn ping_range(ips: &[IpAddr], start_port: u16, end_port: u16) -> NetworkResult<Vec<IpAddr>> {
    let tracker = HostTracker::new();
    let mut alive_ips = Vec::new();
    
    println!("Starting SYN scan of {} IPs across ports {}-{}", 
             ips.len(), start_port, end_port);

    for ip in ips {
        let mut is_alive = false;
        for port in start_port..=end_port {
            let addr = SocketAddr::new(*ip, port);
            
            match syn_scan(addr).await {
                Ok(true) => {
                    is_alive = true;
                    tracker.update_host_status(*ip, true).await;
                    log_alive_host(addr, true).await?;
                    println!("Found open port {}:{}", ip, port);
                    break;
                }
                Ok(false) => continue,
                Err(e) => {
                    eprintln!("Error scanning {}: {}", addr, e);
                    continue;
                }
            }
        }
        
        if !is_alive {
            tracker.update_host_status(*ip, false).await;
        }
        
        // Print current status regardless of state
        tracker.print_status(*ip).await;
        
        if is_alive {
            alive_ips.push(*ip);
        }
    }

    println!("Scan complete. Found {} alive hosts", alive_ips.len());
    Ok(alive_ips)
}

/// Log discovered hosts with timestamp and scan type
async fn log_alive_host(addr: SocketAddr, syn_scan: bool) -> NetworkResult<()> {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let scan_type = if syn_scan { "SYN" } else { "CONNECT" };

    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(LOG_FILE)
        .await
        .map_err(|e| NetworkError::IoError(e))?;

    use tokio::io::AsyncWriteExt;
    file.write_all(format!(
        "[{}] {} scan success: {}:{}\n", 
        timestamp,
        scan_type,
        addr.ip(),
        addr.port()
    ).as_bytes()).await.map_err(|e| NetworkError::IoError(e))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv4Addr;
    use tokio::runtime::Runtime;

    #[test]
    fn test_syn_scan() {
        let rt = Runtime::new().unwrap();
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 80);
        
        rt.block_on(async {
            let result = syn_scan(addr).await;
            assert!(result.is_ok());
        });
    }

    #[test]
    fn test_ping_range() {
        let rt = Runtime::new().unwrap();
        let ips = vec![IpAddr::V4(Ipv4Addr::LOCALHOST)];
        
        rt.block_on(async {
            let alive = ping_range(&ips, 79, 81).await.unwrap();
            assert!(!alive.is_empty());
        });
    }
}