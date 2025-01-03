use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{Mutex, Semaphore};
use tokio::net::{TcpListener, TcpStream};

use crate::core::{
    error::ErrorRegistry,
    types::{AddrData, ServiceDiscovery},
};

pub struct ListenerManager {
    error_registry: Arc<Mutex<ErrorRegistry>>,
    addr_data: Arc<Vec<AddrData>>,
    max_concurrent: usize,
    timeout_duration: Duration,
    max_retries: usize,
    retry_delay: Duration,
    service_discovery: Arc<ServiceDiscovery>,
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