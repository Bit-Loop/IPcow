//! IPCow - High Performance Network Testing Library
//!
//! This library provides core functionality for network testing and analysis:
//! - Multi-threaded TCP/UDP server capabilities
//! - Service discovery and logging
//! - Network error handling and management
//! - Extensible module system for additional features

// Core modules providing fundamental functionality
pub mod core;
// Additional feature modules (ping, web server, etc.)
pub mod modules;
// Utility functions and helpers
pub mod utils;

// Re-export core components
pub use crate::core::CoreConfig;
pub use crate::core::IPCowCore;
pub use crate::core::LogLevel;

// Re-export commonly used types and functions for easier access
pub use crate::core::{
    error::ErrorRegistry,        // Error tracking and management
    handlers::handle_connection, // Connection handling
    network::ListenerManager,    // Multi-threaded listener management
    sockparse::addr_input,       // Address parsing utilities
    types::{AddrData, AddrType}, // Network address type definitions
    ServiceDiscovery,            // Service discovery and logging
};

pub use crate::modules::{ping, web_server}; // Feature modules
pub use crate::utils::helpers; // Utility functions
