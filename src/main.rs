/***********************************************************
 *
 *      IPCow - A simple TCP/UDP Poly Server Written in Rust.
 *          Listen, log, enumerate ports (1 port per thread?)
 *          Send TCP/UDP responses.
 *      Isaiah Tyler Jackson
 *      Created:    Dec 12 2024
 *      Last_ITR:   Dec 16 2024
 *      Version:    00.00.02
 *
 * 
 ***********************************************************/

 #![allow(unused)]
 use std::net::{Ipv4Addr, SocketAddr};
 use std::thread::available_parallelism;
 use tokio::net::{TcpListener, TcpStream};
 use tokio::io::{AsyncReadExt, AsyncWriteExt};
 
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
 
 #[tokio::main]
 async fn main() -> Result<(), Box<dyn std::error::Error>> {
     let system_thread_count = available_parallelism().unwrap().get(); // Get Thread Count for the current system
 
    let default = AddrData {
         info: AddrType::IPv4,
         socket_type: AddrType::UDP,
         address: (192, 168, 1, 16),
         port: 8999,
     };
    let home: AddrData = AddrData { // Example Test assignment
         ..default
     };
 
    let ip_type_text = match home.info {
         AddrType::IPv4 => "It's IPv4 Okay!?",
         AddrType::IPv6 => "WHY ARE YOU USING IPv6??",
         _ => "ITS NOT VALID!!!",
     };
 
    let ip_socket_text = match home.socket_type {
         AddrType::TCP => "TCP.",
         AddrType::UDP => "UDP.",
         _ => "ITS NOT VALID!!!",
     };
 
     print!("\n\nNum of cores: {}\n\n ", system_thread_count);
     println!(
         "Address: {:?}\nPort: {:?},\nAddr Type: {}\nIP Socket Type: {}",
         home.address, home.port, ip_type_text, ip_socket_text
     );
 
     // Using a helper function to construct socket_addressess
     // use simply by socket_address[<num>] to get a valid socket IP and port.
     let socket_address = [
         socket_addr_create(home.address, home.port),
         socket_addr_create(home.address, home.port + 1),
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
 
 // FN Helper to help create socket_address
 fn socket_addr_create(address: (u8, u8, u8, u8), port: u16) -> SocketAddr {
     SocketAddr::from((Ipv4Addr::new(address.0, address.1, address.2, address.3), port))
 }
 