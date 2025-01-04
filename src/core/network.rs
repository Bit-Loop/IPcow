use std::sync::Arc;
use std::time::Duration;
use std::ops::DerefMut;
use tokio::sync::{Mutex, Semaphore};
use tokio::net::TcpListener;

use crate::core::{
    error::ErrorRegistry,
    types::{AddrData, socket_addr_create},
    discovery::ServiceDiscovery,
    handlers::handle_connection,
};

pub struct ListenerManager {
    error_registry: Arc<Mutex<ErrorRegistry>>,
    addr_data: Arc<Vec<AddrData>>,
    max_concurrent: usize,
    service_discovery: Arc<ServiceDiscovery>,
}

impl ListenerManager {
    pub fn new(addr_data: Vec<AddrData>, max_concurrent: usize) -> Self {
        Self {
            error_registry: Arc::new(Mutex::new(ErrorRegistry::new())),
            addr_data: Arc::new(addr_data),
            max_concurrent,
            service_discovery: Arc::new(ServiceDiscovery::new()),
        }
    }

    pub async fn run(&self) -> Result<(), Box<dyn std::error::Error>> {
        let mut listener_tasks = Vec::new();
        let semaphore = Arc::new(Semaphore::new(self.max_concurrent));
        
        for addr_data in self.addr_data.iter() {
            let permit = semaphore.clone().acquire_owned().await?;
            let error_registry = self.error_registry.clone();
            let discovery = self.service_discovery.clone();
            let socket_addr = socket_addr_create(addr_data.address, addr_data.port);
            
            let task = tokio::spawn(async move {
                match TcpListener::bind(&socket_addr).await {
                    Ok(listener) => {
                        println!("Listening on: {}", socket_addr);
                        loop {
                            let accept_result = listener.accept().await;
                            match accept_result {
                                Ok((socket, addr)) => {
                                    let discovery = discovery.clone();
                                    tokio::spawn(async move {
                                        handle_connection(socket, addr, discovery).await;
                                    });
                                }
                                Err(e) => {
                                    let mut registry = error_registry.lock().await;
                                    let error_id = registry.register_error(&e.to_string());
                                    eprintln!("Accept error on {}: ID {}", socket_addr, error_id);
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