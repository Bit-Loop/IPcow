/***********************************************************
 *
 *      IPCow - A simple TCP/UDP Poly Server Written in Rust.
 *          Listen, log, enumerate ports (1 port per thread?)
 *          Send TCP/UDP responses.
 *      Isaiah Tyler Jackson
 *      Created:    Dec 12 2024
 *      Last_ITR:   Dec 16 2024
 *      Version:    00.00.01b
 *
 * 
 ***********************************************************/

//use std::net::{TcpListener, TcpStream};
#![allow(unused)]
//use std::{default, f32::consts::E};
use std::net::{Ipv4Addr, SocketAddr, TcpListener, TcpStream};
use std::thread::available_parallelism;
use std::sync::Arc;
use std::io::{prelude::*, Bytes};
use std::time::Duration;


fn main() -> std::io::Result<()> {
    let system_thread_count = available_parallelism().unwrap().get();
    

    let default = AddrData{
        info: AddrType::IPv4,
        socket_type: AddrType::UDP,
        address: (127,0,0,1),
        port: 22,
    };
    let home:AddrData = AddrData{ // Example Test assignment
        ..default
    };

    let ip_type_text = match home.info{
        AddrType::IPv4 => "It's IPv4 Okay!?",
        AddrType::IPv6 => "WHY ARE YOU USING IPv6??",
        _ => "ITS NOT VALID!!!",
    };

    let ip_socket_text = match home.socket_type{
        AddrType::TCP => "TCP.",
        AddrType::UDP => "UDP.",
        _ => "ITS NOT VALID!!!",
    };

    print!("\n\nNum of cores: {}\n\n ", system_thread_count);
    println!("Address: {:?}\nPort: {:?},\nAddr Type: {}\nIP Socket Type: {}", home.address, home.port, ip_type_text, ip_socket_text);

    // Using a helper function to construct socket_addressess
    // use simply by socket_address[<num>] to get a valid socket IP and port.
    let socket_address = [
        socket_addr_create(home.address, home.port),
        socket_addr_create(home.address, home.port + 1),
    ];

    println!("Socket Addresses: {:?}", socket_address[0]);
    println!("Is IP 0 in socket_address IPv4 using core libs?: {:}", socket_address[0].is_ipv4());

    // The following line returns an error if there is a failure to connect and propagates said error.
    let mut stream_connection = TcpStream::connect(socket_address[0])?;
    // Write a single byte for testing.
    let bytes_written = stream_connection.write(&[0xFF])?;
    stream_connection.flush()?; // Imm write data to stream, ensuring it's sent.
    println!("Wrote {} bytes to the stream", bytes_written);
    // Closed port on IP gives: { code: 111, kind: ConnectionRefused, message: "Connection refused" }
    // Correct/open port gives: { Ok(2) }
    let mut stream_buffer = [0u8; 1024]; // Temp buffer an array of <num> bytes, feeds stream_read_data. Sizes from 1 to 1024 work fine.
    let mut stream_read_data = Vec::new(); // Dynamic buffer to collect data

    loop { // Loop to read all data, itr over it in chunks the size of the buffer.
        
        match stream_connection.read(&mut stream_buffer){
            Ok(0) => { break; } // Nothing to read, nothing to do!
            Ok(bytes_read) => {
                // General NFO  /   DEBUG
                //println!("Stream Bytes Read: {:?}", bytes_read); Always the same amount
                println!("Stream Read Buffer: {:?}", &stream_buffer[..bytes_read]);
                // Append to buffer
                stream_read_data.extend_from_slice(&stream_buffer[..bytes_read]);

                // Make sure we aren't getting \r\n back.
                if stream_read_data.ends_with(&[13,10]){
                    println!("RECIEVED TERMINATION SEQUENCE!");
                    break;
                }
            }
            
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    println!("UH-OH! Looks like a timeout was reached!");
                }
                Err(e) => {
                    eprint!("An unknown error has occoured whilst reading from the stream: {:?}", e);
                    break;
                }
            }
        }




        //let stream_read_byte = stream_connection.read(&mut stream_buffer)?;
        // Append the data read to the dynamic buffe
    
    println!("Quantity of data read: {} bytes.", stream_read_data.len());
    println!("Data as text: {}", String::from_utf8_lossy(&stream_read_data));
    



Ok(())
}

//fn handle_client(stream: TcpStream){
    // .....}

// FN Helper to help create socket_address
fn socket_addr_create(address: (u8,u8,u8,u8), port: u16) -> SocketAddr {
    SocketAddr::from((Ipv4Addr::new(address.0,address.1,address.2,address.3), port))
}
  
  
#[derive(Debug, PartialEq)]
enum AddrType{
    IPv4,
    IPv6,
    TCP,
    UDP,
}

#[derive(Debug)]
struct AddrData {
    info: AddrType,
    socket_type: AddrType,
    address: (u8,u8,u8,u8),
    port: u16,
}
