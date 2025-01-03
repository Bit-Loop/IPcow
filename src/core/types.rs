use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::fmt;

/// Network address types supported by IPCow
#[derive(Debug, PartialEq, Clone)]
pub enum AddrType {
    IPv4,
    IPv6,
    TCP,
    UDP,
}

/// Address data structure containing socket information
#[derive(Debug, Clone)]
pub struct AddrData {
    pub info: AddrType,
    pub socket_type: AddrType,
    pub address: (u8, u8, u8, u8),
    pub port: u16,
}

pub fn socket_addr_create(address: (u8, u8, u8, u8), port: u16) -> SocketAddr {
    SocketAddr::from((
        Ipv4Addr::new(address.0, address.1, address.2, address.3),
        port
    ))
}

/// Connection state for managed connections
#[derive(Debug, Clone)]
pub enum ConnectionState {
    Connected,
    Disconnected,
    Error(String),
}

/// Network configuration settings
#[derive(Debug, Clone)]
pub struct NetworkConfig {
    pub max_connections: usize,
    pub timeout: std::time::Duration,
    pub retry_attempts: u32,
}

/// Custom error type for network operations
#[derive(Debug)]
pub enum NetworkError {
    ConnectionFailed(String),
    InvalidAddress,
    InvalidPort,
    Timeout,
    IoError(std::io::Error),
}

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

impl std::error::Error for NetworkError {}

impl From<std::io::Error> for NetworkError {
    fn from(error: std::io::Error) -> Self {
        NetworkError::IoError(error)
    }
}

/// Result type for network operations
pub type NetworkResult<T> = Result<T, NetworkError>;