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
 * 🚀 Version**:       0.1.1
 * 🛠️  Created-**:      December 12, 2024  
 * 🔄 Last Update**:   Jan 4, 2025  
 * 🧑‍💻 Author:          Isaiah Tyler Jackson  
 *********************************************************************
 */


use clap::{ArgAction, ArgGroup, Parser, Subcommand};
use std::io::{self, Write};
use std::sync::Arc;
use ipcow::{
    AddrType, 
    AddrData, 
    ListenerManager,
    core::{error::ErrorRegistry, sockparse::addr_input},
    utils::helpers::get_thread_factor,
};
use ipcow::modules::*;


/// A high-performance, async TCP server & tool for bug bounty/pentests.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
#[command(group(
    ArgGroup::new("mode")
        .required(false)
        .args(["multi_port_server", "service_discovery", "connection_mgmt", "web_interface", "fuzzing", "performance", "error_registry"]),
))]
struct Cli {
    /// Run the Multi-Port TCP Server module immediately (skips interactive menu)
    #[arg(long, group = "mode", action = ArgAction::SetTrue)]
    multi_port_server: bool,

    /// Run the Service Discovery / Recon module immediately
    #[arg(long, group = "mode", action = ArgAction::SetTrue)]
    service_discovery: bool,

    /// Run the Connection Management Tools module immediately
    #[arg(long, group = "mode", action = ArgAction::SetTrue)]
    connection_mgmt: bool,

    /// Run the Web Interface (UI) module immediately
    #[arg(long, group = "mode", action = ArgAction::SetTrue)]
    web_interface: bool,

    /// Run the Fuzzing & Traffic Analysis module immediately
    #[arg(long, group = "mode", action = ArgAction::SetTrue)]
    fuzzing: bool,

    /// Run the Performance & Metrics module immediately
    #[arg(long, group = "mode", action = ArgAction::SetTrue)]
    performance: bool,

    /// Run the Error Registry & Logging module immediately
    #[arg(long, group = "mode", action = ArgAction::SetTrue)]
    error_registry: bool,

    /// Optional subcommands if you want more structured CLI
    #[command(subcommand)]
    command: Option<Commands>,
}

/// Example subcommands (optional):
#[derive(Subcommand, Debug)]
enum Commands {
    /// Example subcommand to show usage
    ExampleSub,
}

 fn main(){
    let cli = Cli::parse();

    
    // If a subcommand was provided:
    if let Some(cmd) = cli.command {
        match cmd {
            Commands::ExampleSub => {
                println!("You invoked the 'example-sub' subcommand!");
                return;
            }
        }
    }

    // If user passed a specific --<module> flag, skip the interactive menu:
    if cli.multi_port_server {
        start_multi_port_server();
        return;
    }
    if cli.service_discovery {
        run_service_discovery();
        return;
    }
    if cli.connection_mgmt {
        manage_connections();
        return;
    }
    if cli.web_interface {
        start_web_interface();
        return;
    }
    if cli.fuzzing {
        run_fuzzing_module();
        return;
    }
    if cli.performance {
        run_performance_metrics();
        return;
    }
    if cli.error_registry {
        run_error_registry();
        return;
    }

    // Otherwise, display the interactive menu
    loop {
        print_main_menu();
        let choice = prompt_user("> ");

        match choice.trim() {
            "1" => {
                start_multi_port_server();
            }
            "2" => {
                run_service_discovery();
            }
            "3" => {
                manage_connections();
            }
            "4" => {
                start_web_interface();
            }
            "5" => {
                run_fuzzing_module();
            }
            "6" => {
                show_performance_metrics();
            }
            "7" => {
                show_error_registry();
            }
            "9" => {
                println!("Exiting IPCow. Goodbye!");
                break;
            }
            _ => {
                println!("Invalid choice. Please try again.");
            }
        }
    }
}

// -------------------------------
// Helper menu printing & functions
// -------------------------------

fn print_main_menu() {
    println!("\n====== IPCow Main Menu ======");
    println!("1) Start Multi-Port TCP Server");
    println!("2) Service Discovery / Recon");
    println!("3) Connection Management Tools");
    println!("4) Web Interface / Dashboard");
    println!("5) Fuzzing & Traffic Analysis");
    println!("6) Performance & Metrics");
    println!("7) Error Registry & Logging");
    println!("9) Exit");
}

fn prompt_user(prompt: &str) -> String {
    print!("{}", prompt);
    // Flush stdout so the prompt appears immediately
    io::stdout().flush().unwrap();

    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .expect("Failed to read line");
    input
}

// -------------------------------
// Mock module implementations
// -------------------------------

/// Initializes networking components and starts the listener manager
#[tokio::main]
async fn start_multi_port_server() -> Result<(), Box<dyn std::error::Error>> {
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

    // Wait for the manager to complete and return the result
    ///wait_enter();
    manager_handle.await?;
    Ok(())
}


fn run_service_discovery() {
    println!("\n[IPCow] Running Service Discovery / Recon...");
    // TODO: real scanning or discovery logic
    println!("(Stub) Service discovery done. Press ENTER to return.");
    wait_enter();
}

fn manage_connections() {
    println!("\n[IPCow] Opening Connection Management Tools...");
    // TODO: Implement timeouts, graceful shutdown, etc.
    println!("(Stub) Connection management. Press ENTER to return.");
    wait_enter();
}

#[tokio::main]
async fn start_web_interface() {
    println!("\n[IPCow] [WIP:3030]Launching Web Interface / Dashboard...");
    
    let web_server_handle = tokio::spawn(async {
        web_server::run_web_server().await;
    });

    println!("(Stub) Web interface started. Press ENTER to return.");
    wait_enter();
    // Abort the web server task when returning
    web_server_handle.abort();
}

fn run_fuzzing_module() {
    println!("\n[IPCow] Starting Fuzzing & Traffic Analysis...");
    // TODO: Fuzzing logic, custom payload injection
    println!("(Stub) Fuzzing completed. Press ENTER to return.");
    wait_enter();
}

fn run_performance_metrics() {
    println!("\n[IPCow] Gathering Performance & Metrics...");
    // TODO: concurrency tests, resource usage stats
    println!("(Stub) Performance metrics done. Press ENTER to return.");
    wait_enter();
}

fn run_error_registry() {
    println!("\n[IPCow] Opening Error Registry & Logging...");
    // TODO: Show or manage deduplicated errors, correlation, etc.
    println!("(Stub) Error registry. Press ENTER to return.");
    wait_enter();
}

fn wait_enter() {
    let mut input = String::new();
    io::stdin().read_line(&mut input).expect("Failed to read line");
}

fn show_performance_metrics() {
    println!("\n[IPCow] Displaying Performance & Metrics...");
    // TODO: Implement performance monitoring
    println!("(Stub) Performance metrics shown. Press ENTER to return.");
    wait_enter();
}

fn show_error_registry() {
    println!("\n[IPCow] Opening Error Registry & Logging...");
    // TODO: Implement error logging system
    println!("(Stub) Error registry displayed. Press ENTER to return.");
    wait_enter();
}
