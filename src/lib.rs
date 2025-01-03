//! IPCow - High Performance Network Testing Library
//! 
//! Core functionality for network testing and analysis


// Module declarations
pub mod core;
pub mod modules;
pub mod utils;

pub use crate::core::{
    ServiceDiscovery,
    network::ListenerManager,
    types::{AddrType, AddrData},
    error::ErrorRegistry,
    sockparse::addr_input,
    handlers::handle_connection,
};

pub use crate::modules::{ping, web_server};
pub use crate::utils::helpers;
