use std::net::{Ipv4Addr, SocketAddr};
use std::fmt;

/// Network address types supported by IPCow
// Address type enum for specifying IP and socket protocol versions
#[derive(Debug, PartialEq, Clone)]
pub enum AddrType {
    IPv4,
    IPv6,
    TCP,
    UDP,
}

/// Address data structure containing socket information
/// Combines IP address details and port into a single structure
/// Used throughout the application for network endpoint representation
#[derive(Debug, Clone)]
pub struct AddrData {
    pub info: AddrType,          // IP version (v4/v6)
    pub socket_type: AddrType,   // Socket type (TCP/UDP)
    pub address: (u8, u8, u8, u8), // IPv4 address octets
    pub port: u16,               // Port number
}

// Helper function to create SocketAddr from address components
pub fn socket_addr_create(address: (u8, u8, u8, u8), port: u16) -> SocketAddr {
    SocketAddr::from((
        Ipv4Addr::new(address.0, address.1, address.2, address.3),
        port
    ))
}

/// Connection state for managed connections
/// Tracks the current status of network connections
#[derive(Debug, Clone)]
pub enum ConnectionState {
    Connected,                // Active connection
    Disconnected,            // Terminated connection
    Error(String),           // Failed connection with error message
}

/// Network configuration settings
/// Contains tunable parameters for connection management
#[derive(Debug, Clone)]
pub struct NetworkConfig {
    pub max_connections: usize,           // Maximum concurrent connections
    pub timeout: std::time::Duration,     // Connection/operation timeout
    pub retry_attempts: u32,              // Number of retry attempts
}

/// Custom error type for network operations
/// Provides detailed error information for network-related failures
#[derive(Debug)]
pub enum NetworkError {
    ConnectionFailed(String),    // Connection establishment failed
    InvalidAddress,              // Malformed/invalid IP address
    InvalidPort,                 // Invalid port number
    Timeout,                     // Operation timeout
    IoError(std::io::Error),    // Underlying IO error
}

// Implementation of Display trait for NetworkError
// Provides human-readable error messages
impl fmt::Display for NetworkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NetworkError::ConnectionFailed(msg) => write!(f, "Connection failed: {}", msg),
            NetworkError::InvalidAddress => write!(f, "Invalid address"),
            NetworkError::InvalidPort => write!(f, "Invalid port"),
            NetworkError::Timeout => write!(f, "Operation timed out"),
            NetworkError::IoError(e) => write!(f, "IO error: {}", e),
        }
    }
}

// Implement standard error trait for NetworkError
impl std::error::Error for NetworkError {}

impl From<std::io::Error> for NetworkError {
    fn from(error: std::io::Error) -> Self {
        NetworkError::IoError(error)
    }

}

/// Result type for network operations
pub type NetworkResult<T> = Result<T, NetworkError>;