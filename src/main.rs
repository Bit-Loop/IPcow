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
 *          cargo test --test network_tests
 *          Although the tests are not fully implemented yet and are rather basic placeholders.
 *
 * 🚀 Version:         0.1.1
 * 🛠️  Created:        December 12, 2024  
 * 🔄 Last Update:     Jan 6, 2025  
 * 🧑‍💻 Author:        Isaiah Tyler Jackson  
 *********************************************************************
 */

use clap::{ArgAction, ArgGroup, Parser, Subcommand};
use ipcow::core::IPCowCore;
use ipcow::modules::*;
use ipcow::{
    core::{error::ErrorRegistry, sockparse::addr_input, ascii_cube::{display_rotating_cube}},
    utils::helpers::get_thread_factor,
    AddrData, AddrType, ListenerManager,
};
use std::io::{self, Write};
use std::sync::Arc;

/// A high-performance, async TCP server & tool for bug bounty/pentests.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
#[command(group(
    ArgGroup::new("mode")
        .required(false)
        .args(["multi_port_server", "service_discovery", "connection_mgmt", "web_interface", "fuzzing", "performance", "error_registry", "test_network"]),
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

    /// Run network tests
    #[arg(long, group = "mode", action = ArgAction::SetTrue)]
    test_network: bool,

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

fn main() {
    let cli = Cli::parse();

    if let Some(cmd) = cli.command {
        match cmd {
            Commands::ExampleSub => {
                println!("You invoked the 'example-sub' subcommand!");
                return;
            }
        }
    }

    // Handle direct module invocations
    if cli.multi_port_server {
        let _ = start_multi_port_server();
        return;
    }
    if cli.service_discovery {
        let _ = run_service_discovery();
        return;
    }
    if cli.connection_mgmt {
        let _ = manage_connections();
        return;
    }
    if cli.web_interface {
        let _ = start_web_interface();
        return;
    }
    if cli.fuzzing {
        let _ = run_fuzzing_module();
        return;
    }
    if cli.performance {
        let _ = run_performance_metrics();
        return;
    }
    if cli.error_registry {
        let _ = run_error_registry();
        return;
    }
    if cli.test_network {
        let _ = run_network_tests();
        return;
    }

    // Interactive menu loop
    loop {
        print_main_menu();
        match prompt_user("> ").trim() {
            "1" => {
                let _ = start_multi_port_server();
            }
            "2" => {
                let _ = run_service_discovery();
            }
            "3" => {
                let _ = manage_connections();
            }
            "4" => {
                let _ = start_web_interface();
            }
            "5" => {
                let _ = run_fuzzing_module();
            }
            "6" => {
                let _ = show_performance_metrics();
            }
            "7" => {
                let _ = show_error_registry();
            }
            "8" => {
                let _ = display_rotating_cube();
            }
            "9" => {
                println!("Exiting IPCow. Goodbye!");
                break;
            }
            _ => println!("Invalid choice. Please try again."),
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
    println!("8) TEST ASCII Animation");
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
    println!("\n[IPCow] Starting Multi-Port TCP Server...");

    let core = IPCowCore::new();
    let max_workers = get_thread_factor();
    let (ips_vec, ports_vec) = addr_input();

    let ips: Arc<Vec<std::net::IpAddr>> =
        Arc::new(ips_vec.into_iter().map(std::net::IpAddr::V4).collect());
    let ports: Arc<Vec<u16>> = Arc::new(ports_vec);

    println!("\nServer Configuration:");
    println!("- Worker threads: {}", max_workers);
    println!("- IP addresses: {}", ips.len());
    println!("- Ports per IP: {}", ports.len());

    let addr_data_list: Vec<AddrData> = ips
        .iter()
        .flat_map(|ip| {
            ports.iter().map(move |port| AddrData {
                info: AddrType::IPv4,
                socket_type: AddrType::TCP,
                address: match ip {
                    std::net::IpAddr::V4(ipv4) => ipv4.octets().into(),
                    _ => panic!("IPv6 not supported"),
                },
                port: *port,
            })
        })
        .collect();

    println!("- Total listeners: {}", addr_data_list.len());

    {
        let mut network_manager = core.network_manager.lock().await;
        *network_manager = ListenerManager::new(addr_data_list, max_workers);
    }

    println!("\nPress Ctrl+C to stop the server...\n");
    core.start().await?;

    Ok(())
}

fn run_service_discovery() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n[IPCow] Running Service Discovery / Recon...");
    // TODO: real scanning or discovery logic
    println!("(Stub) Service discovery done. Press ENTER to return.");
    wait_enter();
    Ok(())
}

fn manage_connections() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n[IPCow] Opening Connection Management Tools...");
    // TODO: Implement timeouts, graceful shutdown, etc.
    println!("(Stub) Connection management. Press ENTER to return.");
    wait_enter();
    Ok(())
}

#[tokio::main]
async fn start_web_interface() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n[IPCow] [WIP:3030]Launching Web Interface / Dashboard...");
    web_server::run_web_server().await;
    Ok(())
}

fn run_fuzzing_module() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n[IPCow] Starting Fuzzing & Traffic Analysis...");
    // TODO: Fuzzing logic, custom payload injection
    println!("(Stub) Fuzzing completed. Press ENTER to return.");
    wait_enter();
    Ok(())
}

fn run_performance_metrics() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n[IPCow] Gathering Performance & Metrics...");
    // TODO: concurrency tests, resource usage stats
    println!("(Stub) Performance metrics done. Press ENTER to return.");
    wait_enter();
    Ok(())
}

fn run_error_registry() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n[IPCow] Opening Error Registry & Logging...");
    // TODO: Show or manage deduplicated errors, correlation, etc.
    println!("(Stub) Error registry. Press ENTER to return.");
    wait_enter();
    Ok(())
}

fn wait_enter() {
    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .expect("Failed to read line");
}

fn show_performance_metrics() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n[IPCow] Displaying Performance & Metrics...");
    // TODO: Implement performance monitoring
    println!("(Stub) Performance metrics shown. Press ENTER to return.");
    wait_enter();
    Ok(())
}

fn show_error_registry() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n[IPCow] Opening Error Registry & Logging...");
    // TODO: Implement error logging system
    println!("(Stub) Error registry displayed. Press ENTER to return.");
    wait_enter();
    Ok(())
}

#[tokio::main]
async fn run_network_tests() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n[IPCow] Running Network Tests...");
    
    // Test local connectivity
    let local_ports = vec![80, 443, 8080];
    println!("Testing local ports: {:?}", local_ports);
    
    for port in local_ports {
        let addr = format!("127.0.0.1:{}", port);
        match tokio::net::TcpStream::connect(&addr).await {
            Ok(_) => println!("✅ Port {} is open", port),
            Err(_) => println!("❌ Port {} is closed", port),
        }
    }

    // Test DNS resolution
    println!("\nTesting DNS resolution...");
    let domains = vec!["google.com", "github.com", "example.com"];
    for domain in domains {
        match tokio::net::lookup_host(format!("{}:80", domain)).await {
            Ok(addrs) => println!("✅ {} resolves to: {:?}", domain, addrs.collect::<Vec<_>>()),
            Err(e) => println!("❌ Failed to resolve {}: {}", domain, e),
        }
    }

    // Test network latency
    println!("\nTesting network latency...");
    let targets = vec!["1.1.1.1:53", "8.8.8.8:53"];
    for target in targets {
        let start = std::time::Instant::now();
        match tokio::net::TcpStream::connect(target).await {
            Ok(_) => println!("✅ {} latency: {:?}", target, start.elapsed()),
            Err(e) => println!("❌ Failed to connect to {}: {}", target, e),
        }
    }

    println!("\nNetwork tests complete. Press ENTER to return.");
    wait_enter();
    Ok(())
}
