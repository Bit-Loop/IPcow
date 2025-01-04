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
 * 🚀 Version**:       0.0.3  
* 🛠️  Created-**:      December 12, 2024  
 * 🔄 Last Update**:   Jan 2, 2025  
 * 🧑‍💻 Author:          Isaiah Tyler Jackson  
 *********************************************************************
 */
use std::{
    io::{self, stdin, stdout, Write},
    net::{Ipv4Addr, SocketAddr},
    num::NonZeroUsize,
    sync::{atomic::AtomicUsize, Arc},
    thread::available_parallelism,
    collections::HashMap,
    hash::{Hash, Hasher},
    collections::hash_map::DefaultHasher,
};

use futures::stream::StreamExt;
use itertools::Itertools;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    sync::{Mutex, Semaphore},
    time::{timeout, Duration},
};

use sockparse::{addr_input};


pub struct ListenerManager {
    error_registry: Arc<Mutex<ErrorRegistry>>,
    addr_data: Arc<Vec<AddrData>>,
    max_concurrent: usize,
    timeout_duration: Duration,
}

impl ListenerManager {
    fn new(addr_data: Vec<AddrData>, max_concurrent: usize) -> Self {
        Self {
            error_registry: Arc::new(Mutex::new(ErrorRegistry::new())),
            addr_data: Arc::new(addr_data),
            max_concurrent,
            timeout_duration: Duration::from_secs(30),
        }
    }

    async fn run(&self) -> Result<(), Box<dyn std::error::Error>> {
        let mut listener_tasks = Vec::new();
        let semaphore = Arc::new(Semaphore::new(self.max_concurrent));

        for addr_data in self.addr_data.iter() {
            let permit = semaphore.clone().acquire_owned().await?;
            let error_registry = self.error_registry.clone();
            let socket_addr = socket_addr_create(addr_data.address, addr_data.port);
            
            let task = tokio::spawn(async move {
                match TcpListener::bind(&socket_addr).await {
                    Ok(listener) => {
                        println!("Listening on: {}", socket_addr);
                        
                        loop {
                            match timeout(self.timeout_duration, listener.accept()).await {
                                Ok(Ok((mut socket, addr))) => {  // Added mut here
                                    tokio::spawn(async move {
                                        let mut stream_buffer = [0u8; 1024];
                                        let mut stream_read_data = Vec::new();
                                        
                                        loop {
                                            match socket.read(&mut stream_buffer).await {
                                                Ok(0) => {
                                                    println!("Connection closed by peer: {:?}", addr);
                                                    break;
                                                }
                                                Ok(bytes_read) => {
                                                    println!("Stream Read Buffer: {:?}", &stream_buffer[..bytes_read]);
                                                    stream_read_data.extend_from_slice(&stream_buffer[..bytes_read]);
                                                    if stream_read_data.ends_with(&[13, 10]) {
                                                        println!("RECEIVED TERMINATION SEQUENCE!");
                                                        break;
                                                    }
                                                    if let Err(e) = socket.write_all(&stream_buffer[..bytes_read]).await {
                                                        eprintln!("Failed to write to socket: {:?}", e);
                                                        break;
                                                    }
                                                }
                                                Err(e) => {
                                                    eprintln!("Error reading from socket: {:?}", e);
                                                    break;
                                                }
                                            }
                                        }
                                    });
                                }
                                Ok(Err(e)) => {
                                    let mut registry = error_registry.lock().await;
                                    let error_id = registry.register_error(&e.to_string());
                                    println!("Accept error on {}: ID {}", socket_addr, error_id);
                                }
                                Err(_) => {
                                    let mut registry = error_registry.lock().await;
                                    let error_id = registry.register_error("Connection timeout");
                                    println!("Timeout on {}: ID {}", socket_addr, error_id);
                                }
                            }
                        }
                    }
                    Err(e) => {
                        let mut registry = error_registry.lock().await;
                        let error_id = registry.register_error(&e.to_string());
                        println!("Bind error on {}: ID {}", socket_addr, error_id);
                    }
                }
                drop(permit);
            });
            
            listener_tasks.push(task);
        }

        futures::future::join_all(listener_tasks).await;
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let system_thread_count = available_parallelism().unwrap().get();
    let max_workers = get_thread_factor();
    //let max_conn = get_max_conn_count();

    let (ips_vec, ports_vec) = addr_input();
    let mut addr_data_list: Vec<AddrData> = Vec::new();
    let ips = Arc::new(ips_vec);
    let ports = Arc::new(ports_vec);

    // Now we create addr_data_list directly:
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

    let addr_data_list = Arc::new(addr_data_list); // Wrap final list in Arc for shared ownership

    let chunk_size = ((addr_data_list.len() + (max_workers - 1)) / max_workers).max(1);
    println!(
        "ADDR DATA LEN: {:?}, Chunk Size: {:?}",
        addr_data_list.len(),
        chunk_size
    );

    let max_concurrent_tasks = 8; // Adjust based on available cores and workload
    let semaphore = Arc::new(Semaphore::new(max_concurrent_tasks));

    let addr_iter = ips.iter().flat_map(move |ip| {
        let ports = Arc::clone(&ports); // Clone `ports` Arc for the closure
        let ports_clone = Arc::clone(&ports); // Clone `Arc` outside the closure
        ports_clone
            .iter()
            .map(move |port| (ip.clone(), *port))
            .collect::<Vec<_>>()
            .into_iter()
    });
    let chunks = addr_iter.chunks(chunk_size);

    let mut tasks = Vec::new();

    for chunk_vec in chunks.into_iter().map(|chunk| chunk.collect::<Vec<_>>()) {
        let permit = semaphore.clone().acquire_owned().await.unwrap();
        for chunk_vec in chunks.into_iter().map(|chunk| chunk.collect::<Vec<_>>()) {
            // Process chunk_vec here
        }
        let task = tokio::spawn(async move {
            for (ip, port) in chunk_vec {
                let address = (
                    ip.octets()[0],
                    ip.octets()[1],
                    ip.octets()[2],
                    ip.octets()[3],
                );
                let addr_data = AddrData {
                    info: AddrType::IPv4,
                    socket_type: AddrType::TCP,
                    address,
                    port,
                };
                process_address(addr_data).await.unwrap();
                // Permit automatically released when scope ends
            }
        });
        tasks.push(task);
    }

    // Wait for all tasks to complete
    futures::future::join_all(tasks).await;

    // Using a helper function to construct socket_addressess
    // use simply by socket_address[<num>] to get a valid socket IP and port.
    let socket_address = [
        socket_addr_create(addr_data_list[0].address, addr_data_list[0].port),
        socket_addr_create(addr_data_list[0].address, addr_data_list[0].port + 1),
    ];

    println!("Socket Addresses: {:?}", socket_address[0]);
    println!(
        "Is IP 0 in socket_address IPv4 using core libs?: {}",
        socket_address[0].is_ipv4()
    );

    // Bind the TCP listener using tokio

    println!("\n\nSocket is: {:?}\n\n", socket_address);

    let manager = ListenerManager::new(addr_data_list.to_vec(), max_workers);
    manager.run().await?;

    Ok(())
}
/*
 *      [   DECLERATIONS   ]
 */

#[derive(Debug, PartialEq, Clone)]
pub enum AddrType {
    IPv4,
    IPv6,
    TCP,
    UDP,
}

#[derive(Debug, Clone)]
pub struct AddrData {
    pub info: AddrType,
    pub socket_type: AddrType,
    pub address: (u8, u8, u8, u8),
    pub port: u16,
}

// FN Helper to help create socket_address
pub fn socket_addr_create(address: (u8, u8, u8, u8), port: u16) -> SocketAddr {
    SocketAddr::from((
        Ipv4Addr::new(address.0, address.1, address.2, address.3),
        port,
    ))
}

enum ChunkState {
    Idle,
    Ready,
    Processing,
    Completed,
    Error,
}

struct Chunk {
    state: ChunkState,
    data: Vec<(String, u16)>, // IP-Port pairs
}

struct SharedState {
    chunk_stack: Mutex<Vec<Arc<Mutex<Chunk>>>>,
    connections_processed: AtomicUsize,
    chunks_completed: AtomicUsize,
    error_log: Mutex<Vec<String>>,
}

/*
 *      [   FUNCTIONS   ]
 */

fn get_thread_factor() -> usize {
    let system_threads = available_parallelism()
        .unwrap_or(NonZeroUsize::new(1).unwrap())
        .get();
    loop {
        print!(
            "System Threads: {}\nEnter a multiplication factor for threads (positive integer): ",
            system_threads
        );
        io::stdout().flush().unwrap();

        let mut input = String::new();
        if let Err(_) = io::stdin().read_line(&mut input) {
            println!("Failed to read input. Please try again.");
            continue;
        }

        if let Ok(factor) = input.trim().parse::<usize>() {
            if factor > 0 {
                let total_threads = system_threads * factor;
                println!(
                    "Using {} threads ({} system threads × {} factor).",
                    total_threads, system_threads, factor
                );
                return total_threads;
            } else {
                println!("Factor must be greater than 0.");
            }
        } else {
            println!("Invalid input. Enter a positive integer.");
        }
    }
}

fn get_max_conn() -> usize {
    use std::fs;

    // Path to the proc file
    let path = "/proc/sys/net/netfilter/nf_conntrack_max";
    let default: usize = 262144;
    let free_alloc = 8192;

    // Try to read the file and parse its contents, falling back to a default value
    let result = match fs::read_to_string(path) {
        Ok(contents) => contents.trim().parse::<usize>().unwrap_or(default), // Use default if parsing fails
        Err(_) => default, // Use default if file read fails
    };

    // Adjust the result by subtracting `free_alloc`
    result.saturating_sub(free_alloc) // Ensures no underflow occur
}

async fn process_address(addr_data: AddrData) -> Result<(), Box<dyn std::error::Error>> {
    use tokio::time::timeout;
    use std::time::Duration;

    const PROCESS_TIMEOUT: Duration = Duration::from_secs(30);

    println!("Starting process for address: {:?}", addr_data);
    
    match timeout(PROCESS_TIMEOUT, async {
        // Put actual processing logic here
        Ok(())
    }).await {
        Ok(result) => result,
        Err(_) => Err("Address processing timed out".into())
    }
}
