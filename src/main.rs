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
            Althogh, the tests are not fully implemented yet and are rather basic as a place holder.
 *
 * 🚀 Version**:       0.1.0
* 🛠️  Created-**:      December 12, 2024  
 * 🔄 Last Update**:   Jan 3, 2025  
 * 🧑‍💻 Author:          Isaiah Tyler Jackson  
 *********************************************************************
 */
use std::{
    io::{self, Write},
    num::NonZeroUsize,
    sync::Arc,
    thread::available_parallelism,
};
use tokio::sync::Semaphore;
use ipcow::{
    AddrType, 
    AddrData, 
    ListenerManager,
    sockparse::addr_input,
};

mod web_server;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let max_workers = get_thread_factor();
    
    // Get IP and port configurations
    let (ips_vec, ports_vec) = addr_input();
    let ips: Arc<Vec<_>> = Arc::new(ips_vec);
    let ports: Arc<Vec<_>> = Arc::new(ports_vec);

    // Create address data list
    let addr_data_list: Vec<AddrData> = ips
        .iter()
        .flat_map(|ip| {
            ports.iter().map(move |port| AddrData {
                info: AddrType::IPv4,
                socket_type: AddrType::TCP,
                address: ip.octets().into(),
                port: *port,
            })
        })
        .collect();

    // Create and run the listener manager
    let manager = ListenerManager::new(addr_data_list, max_workers);
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

// Keep utility functions specific to the binary
fn get_thread_factor() -> usize {
    let system_threads = available_parallelism()
        .unwrap_or(NonZeroUsize::new(1).unwrap())
        .get();
    
    loop {
        print!(
            "System Threads: {}\nEnter thread multiplier: ",
            system_threads
        );
        io::stdout().flush().unwrap();

        let mut input = String::new();
        if io::stdin().read_line(&mut input).is_ok() {
            if let Ok(factor) = input.trim().parse::<usize>() {
                if factor > 0 {
                    return system_threads * factor;
                }
            }
        }
        println!("Please enter a positive number");
    }
}
