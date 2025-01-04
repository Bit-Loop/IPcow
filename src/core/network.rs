// Network management module handling TCP listener initialization and connection handling
use std::sync::Arc;
use tokio::sync::{Mutex, Semaphore};
use tokio::net::TcpListener;

use crate::core::{
    error::ErrorRegistry,
    types::{AddrData, socket_addr_create},
    discovery::ServiceDiscovery, 
    handlers::handle_connection,
};

/// Main struct responsible for managing multiple TCP listeners
/// Handles concurrent connections and service discovery across multiple ports
pub struct ListenerManager {
    // Shared error tracking system
    error_registry: Arc<Mutex<ErrorRegistry>>,
    // Vector of IP/Port combinations to listen on
    addr_data: Arc<Vec<AddrData>>,
    // Maximum number of concurrent connections allowed
    max_concurrent: usize,
    // Service detection and tracking system
    service_discovery: Arc<ServiceDiscovery>,
}

impl ListenerManager {
    /// Creates a new ListenerManager instance
    /// Sets up error registry, connection limits, and service discovery
    pub fn new(addr_data: Vec<AddrData>, max_concurrent: usize) -> Self {
        Self {
            error_registry: Arc::new(Mutex::new(ErrorRegistry::new())),
            addr_data: Arc::new(addr_data),
            max_concurrent,
            service_discovery: Arc::new(ServiceDiscovery::new()),
        }
    }

    /// Main entry point for starting TCP listeners
    /// Spawns async tasks for each address/port combination
    pub async fn run(&self) -> Result<(), Box<dyn std::error::Error>> {
        // Track spawned listener tasks
        let mut listener_tasks = Vec::new();
        // Limit concurrent connections
        let semaphore = Arc::new(Semaphore::new(self.max_concurrent));
        
        // Iterate through each address/port combination
        for addr_data in self.addr_data.iter() {
            // Acquire permission to create new listener
            let permit = semaphore.clone().acquire_owned().await?;
            let error_registry = self.error_registry.clone();
            let discovery = self.service_discovery.clone();
            let socket_addr = socket_addr_create(addr_data.address, addr_data.port);
            
            // Spawn individual listener task
            let task = tokio::spawn(async move {
                match TcpListener::bind(&socket_addr).await {
                    Ok(listener) => {
                        println!("Listening on: {}", socket_addr);
                        // Accept loop for handling incoming connections
                        loop {
                            let accept_result = listener.accept().await;
                            match accept_result {
                                Ok((socket, addr)) => {
                                    // Spawn task for each accepted connection
                                    let discovery = discovery.clone();
                                    tokio::spawn(async move {
                                        handle_connection(socket, addr, discovery).await;
                                    });
                                }
                                Err(e) => {
                                    // Log accept errors with unique ID
                                    let mut registry = error_registry.lock().await;
                                    let error_id = registry.register_error(&e.to_string());
                                    eprintln!("Accept error on {}: ID {}", socket_addr, error_id);
                                }
                            }
                        }
                    }
                    Err(e) => {
                        // Log bind errors with unique ID
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