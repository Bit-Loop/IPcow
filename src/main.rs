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
 * 🚀 Version**:       0.0.2  
* 🛠️  Created-**:      December 12, 2024  
 * 🔄 Last Update**:   December 19, 2024  
 * 🧑‍💻 Author:          Isaiah Tyler Jackson  
 *********************************************************************
 */

#![allow(unused)]
use std::default;
use std::net::{Ipv4Addr, SocketAddr};
use std::sync::atomic::AtomicUsize;
use std::sync::{Arc, Mutex};
use std::thread::available_parallelism;
use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use std::io::{self, stdin, stdout, Write};
use sockparse::{addr_input,parse_ip_input}; 
use std::num::NonZeroUsize;
use tokio::sync::Semaphore;
use futures::stream::StreamExt; // For `for_each_concurrent`
use itertools::Itertools; // For `chunks`



 #[tokio::main]
 async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let system_thread_count = available_parallelism().unwrap().get(); // Get Thread Count for the current system
    let max_workers = get_thread_factor();
    //let max_conn = get_max_conn_count();

    let (ips_vec, ports_vec) = addr_input(); // Returns a tuple (Vec<Ipv4Addr>, Vec<u16>)
    let ips = Arc::new(ips_vec); // Wrap `ips` in Arc for shared ownership
    let ports = Arc::new(ports_vec); // Wrap `ports` in Arc for shared ownership
    let mut addr_data_list: Vec<AddrData> = Vec::new(); // vector of <enum struct>
    // Assemble the addr list. Iterate over all IP and port combinations
		
    let chunk_size = (addr_data_list.len() + (max_workers -1))/max_workers; 
    println!("ADDR DATA LEN: {:?}, Chunk Size: {:?}", addr_data_list.len(), chunk_size);


    let max_concurrent_tasks = 8; // Adjust based on available cores and workload
    let semaphore = Arc::new(Semaphore::new(max_concurrent_tasks));

    let addr_iter = ips.iter().flat_map(move |ip| {
        let ports = Arc::clone(&ports); // Clone `ports` Arc for the closure
        ports.iter().map(move |port| (ip.clone(), port.clone()))
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
                    port: port,
                };
                process_address(addr_data).await;
                // Permit automatically released when scope ends
            }
        });
        tasks.push(task);
    }

    // Wait for all tasks to complete
    futures::future::join_all(tasks).await;

    let ip_type_text = match addr_data_list[0].info {
         AddrType::IPv4 => "It's IPv4 Okay!?",
         AddrType::IPv6 => "WHY ARE YOU USING IPv6??",
         _ => "ITS NOT VALID!!!",
     };
    let ip_socket_text = match addr_data_list[0].socket_type {
         AddrType::TCP => "TCP.",
         AddrType::UDP => "UDP.",
         _ => "ITS NOT VALID!!!",
     }; 
     print!("\n\nNum of cores: {}\n\n", system_thread_count);
     println!(
         "Address: {:?}\nPort: {:?},\nAddr Type: {}\nIP Socket Type: {}",
         addr_data_list[0].address,  addr_data_list[0].port, ip_type_text, ip_socket_text
     );
 
     // Using a helper function to construct socket_addressess
     // use simply by socket_address[<num>] to get a valid socket IP and port.
     let socket_address = [
         socket_addr_create( addr_data_list[0].address,  addr_data_list[0].port),
         socket_addr_create( addr_data_list[0].address,  addr_data_list[0].port + 1),
     ];
 
     println!("Socket Addresses: {:?}", socket_address[0]);
     println!(
         "Is IP 0 in socket_address IPv4 using core libs?: {}",
         socket_address[0].is_ipv4()
     );
 
     // Bind the TCP listener using tokio
     
    println!("\n\nSocket is: {:?}\n\n", socket_address);

     let listener = TcpListener::bind(socket_address[0]).await?;
     println!("Listening on: {}", socket_address[0]);
     println!("\n\nTCP listener is: {:?}\n\n", listener);

     // Accept connections in a loop
     loop {
         let (mut socket, addr) = listener.accept().await?;
         println!("New connection: {:?}", addr);
 
         tokio::spawn(async move {
             let mut stream_buffer = [0u8; 1024]; // Temp buffer: array of <num> bytes, feeds stream_read_data
             let mut stream_read_data = Vec::new(); // Dynamic buffer to collect data
 
             loop { // Loop to read all data, iterate over it in chunks of the buffer size
                 match socket.read(&mut stream_buffer).await {
                     Ok(0) => {
                         println!("Connection closed by peer: {:?}", addr);
                         break;
                     } // Nothing to read, nothing to do!
                     Ok(bytes_read) => {
                         // General INFO / DEBUG
                         println!(
                             "Stream Read Buffer: {:?}",
                             &stream_buffer[..bytes_read]
                         );
 
                         // Append to dynamic buffer
                         stream_read_data.extend_from_slice(&stream_buffer[..bytes_read]);
 
                         // Check for termination sequence (\r\n)
                         if stream_read_data.ends_with(&[13, 10]) {
                             println!("RECEIVED TERMINATION SEQUENCE!");
                             break;
                         }
 
                         // Echo data back to the client
                         if let Err(e) = socket.write_all(&stream_buffer[..bytes_read]).await {
                             eprintln!("Failed to write back to socket: {:?}", e);
                             break;
                         }
                     }
                     Err(e) => {
                         eprintln!("Error reading from socket: {:?}", e);
                         break;
                     }
                 }
             }
 
             println!(
                 "Quantity of data read: {} bytes.",
                 stream_read_data.len()
             );
             println!(
                 "Data as text: {}",
                 String::from_utf8_lossy(&stream_read_data)
             );
         });
     }
 }
 /*
 *      [   DECLERATIONS   ]
 */
 #[derive(Debug, PartialEq)]
 enum AddrType {
     IPv4,
     IPv6,
     TCP,
     UDP,
 }
 
 #[derive(Debug)]
 struct AddrData {
     info: AddrType,
     socket_type: AddrType,
     address: (u8, u8, u8, u8),
     port: u16,
 }

 // FN Helper to help create socket_address
 fn socket_addr_create(address: (u8, u8, u8, u8), port: u16) -> SocketAddr {
     SocketAddr::from((Ipv4Addr::new(address.0, address.1, address.2, address.3), port))
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
    data: Vec<(String,u16)>, // IP-Port pairs
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

fn get_max_conn() -> usize{
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
    
async fn process_address(addr_data: AddrData) {
    println!("Processing address: {:?}", addr_data);
    // Add more logic for processing as needed
}

