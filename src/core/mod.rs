pub mod discovery;
pub mod error;
pub mod handlers;
pub mod network;
pub mod sockparse;
pub mod state;
pub mod types;
pub mod ascii_cube;

use std::sync::Arc;
use tokio::sync::Mutex;

pub use ascii_cube::AsciiCube;
pub use ascii_cube::display_rotating_cube;


// Core configuration settings
#[derive(Debug)]
pub struct CoreConfig {
    pub max_workers: usize,
    pub web_port: u16,
    pub log_level: LogLevel,
}

#[derive(Debug, Clone, Copy)]
pub enum LogLevel {
    Debug,
    Info,
    Warning,
    Error,
}

// Main core struct managing all components
pub struct IPCowCore {
    // Shared state
    pub state: Arc<Mutex<state::CoreState>>,

    // Core managers
    pub network_manager: Arc<Mutex<network::ListenerManager>>,
    pub discovery_manager: Arc<Mutex<discovery::ServiceDiscovery>>,
    pub error_manager: Arc<Mutex<error::ErrorRegistry>>,

    // Configuration
    pub config: CoreConfig,
}

impl IPCowCore {
    // Constructor with default configuration
    pub fn new() -> Self {
        Self::with_config(CoreConfig {
            max_workers: 4,
            web_port: 3030,
            log_level: LogLevel::Info,
        })
    }

    // Constructor with custom configuration
    pub fn with_config(config: CoreConfig) -> Self {
        Self {
            state: Arc::new(Mutex::new(state::CoreState::new())),
            network_manager: Arc::new(Mutex::new(network::ListenerManager::new(
                vec![],
                config.max_workers,
            ))),
            discovery_manager: Arc::new(Mutex::new(discovery::ServiceDiscovery::new())),
            error_manager: Arc::new(Mutex::new(error::ErrorRegistry::new())),
            config,
        }
    }

    // Core lifecycle methods
    pub async fn start(&self) -> Result<(), Box<dyn std::error::Error>> {
        println!("[Core] Starting IPCow core services...");

        // Start network manager
        let network = self.network_manager.lock().await;
        network.run().await?;

        // Set running state
        let mut state = self.state.lock().await;
        state.is_running = true;

        Ok(())
    }

    pub async fn shutdown(&self) -> Result<(), Box<dyn std::error::Error>> {
        println!("[Core] Shutting down IPCow core services...");

        let mut state = self.state.lock().await;
        state.is_running = false;

        Ok(())
    }
}

// Re-exporting commonly used components
pub use discovery::ServiceDiscovery;
pub use error::ErrorRegistry;
pub use handlers::handle_connection;
pub use network::ListenerManager;
pub use sockparse::addr_input;
pub use types::{AddrData, AddrType};
