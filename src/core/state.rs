use crate::core::types::{ConnectionState, NetworkConfig};
use std::collections::HashMap;
use std::net::SocketAddr;

pub struct CoreState {
    pub active_connections: HashMap<SocketAddr, ConnectionState>,
    pub network_config: NetworkConfig,
    pub is_running: bool,
}

impl CoreState {
    pub fn new() -> Self {
        Self {
            active_connections: HashMap::new(),
            network_config: NetworkConfig {
                max_connections: 1000,
                timeout: std::time::Duration::from_secs(30),
                retry_attempts: 3,
            },
            is_running: false,
        }
    }

    pub fn update_connection(&mut self, addr: SocketAddr, state: ConnectionState) {
        self.active_connections.insert(addr, state);
    }

    pub fn get_active_connections(&self) -> Vec<(SocketAddr, ConnectionState)> {
        self.active_connections
            .iter()
            .map(|(k, v)| (*k, v.clone()))
            .collect()
    }
}
