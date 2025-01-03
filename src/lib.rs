//! IPCow - High Performance Network Testing Library
//! 
//! Core functionality for network testing and analysis

// Standard library imports
use std::{
    collections::{HashMap, hash_map::DefaultHasher},
    fs::OpenOptions,
    hash::{Hash, Hasher},
    io::Write,
    net::{Ipv4Addr, SocketAddr},
    path::PathBuf,
    sync::Arc,
    time::Duration,
};

// External crate imports
use chrono::Local;
use tokio::{
    sync::{Mutex, Semaphore},
    net::{TcpListener, TcpStream},
    io::{AsyncReadExt, AsyncWriteExt},
};

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
};

pub use crate::modules::{ping, web_server, handlers::handle_connection};
pub use crate::utils::helpers;
