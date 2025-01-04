/*!
 *********************************************************************
 *                           🐮 IPCow 🐮                             
 *       A **simple, robust TCP/UDP Poly Server** written in Rust.  
 * ------------------------------------------------------------------
 * 📡 **Features**:
 *   - Listen on multiple IP addresses and ports.
 *   - Log incoming connections and traffic.
 *   - Enumerate and manage ports (1 port per thread, optimized).
 *   - Send TCP/UDP responses seamlessly.
 *
 * 🔧 **Designed For**:
 *   - Flexibility in multi-address and multi-port setups.
 *   - High performance through parallelization.
 *   - Ease of integration into any Rust-based networking environment.
 *  
 *  [Note: This is a work-in-progress project.]
 *      You can do basic testing with:
 *          cargo test --test system_tests
 *           cargo test --test network_tests
 *           Althogh, the tests are not fully implemented yet and are rather basic as a place holder.
 *
 * 🚀 Version**:       0.1.0
 * 🛠️  Created-**:      December 12, 2024  
 * 🔄 Last Update**:   Jan 3, 2025  
 * 🧑‍💻 Author:          Isaiah Tyler Jackson  
 *********************************************************************
 */

// Import required dependencies for network and concurrency handling
use std::sync::Arc;
use ipcow::{
    AddrType, 
    AddrData, 
    ListenerManager,
    core::{error::ErrorRegistry, sockparse::addr_input},
    utils::helpers::get_thread_factor,
};
use ipcow::modules::*;

/// Main entry point for the IPCow server
/// Initializes networking components and starts the listener manager
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Calculate optimal number of worker threads based on system capabilities
    let max_workers = get_thread_factor();

    // Get user-configured IP addresses and ports for listening
    let (ips_vec, ports_vec) = addr_input();
    // Convert vectors to the expected types before wrapping in Arc
    let ips: Arc<Vec<std::net::IpAddr>> = Arc::new(ips_vec.into_iter().map(std::net::IpAddr::V4).collect());
    let ports: Arc<Vec<u16>> = Arc::new(ports_vec);

    // Generate all possible IP:Port combinations for listening
    // Creates a flat list of address configurations for the server
    let addr_data_list: Vec<AddrData> = ips
        .iter()
        .flat_map(|ip| {
            ports.iter().map(move |port| AddrData {
                info: AddrType::IPv4,          // IPv4 address type
                socket_type: AddrType::TCP,    // TCP socket type
                address: match ip {
                    std::net::IpAddr::V4(ipv4) => ipv4.octets().into(),
                    _ => panic!("IPv6 not supported"),
                },   // Convert IP to tuple
                port: *port,                   // Assign port number
            })
        })
        .collect();

    // Initialize the main listener manager with configurations
    let manager = ListenerManager::new(addr_data_list, max_workers);
    // Spawn the manager in a separate task for concurrent operation
    let manager_handle = tokio::spawn(async move {
        manager.run().await.unwrap();
    });

    // Run the web server
    let web_server_handle = tokio::spawn(async {
        web_server::run_web_server().await;
    });

    // Wait for both tasks to complete
    tokio::try_join!(manager_handle, web_server_handle)?;

    Ok(())
}
