/***********************************************************
 *
 *      IPCow - A simple TCP/UDP Poly Server Written in Rust.
 *          Listen, log, enumerate ports (1 port per thread?)
 *          Send TCP/UDP responses.
 *      Isaiah Tyler Jackson
 *      Created:    Dec 12 2024
 *      Last_ITR:   Dec 14 2024
 *
 * 
 ***********************************************************/

//use std::net::{TcpListener, TcpStream};
#![allow(unused)]
//use std::{default, f32::consts::E};
use std::net::{Ipv4Addr, SocketAddr, TcpListener, TcpStream};
use std::thread::available_parallelism;
use std::sync::Arc;
use std::io::prelude::*;

  
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


fn main() -> std::io::Result<()> {
    let system_thread_count = available_parallelism().unwrap().get();
    

    let default = AddrData{
        info: AddrType::IPv4,
        socket_type: AddrType::UDP,
        address: (127,0,0,1),
        port: 80,
    };
    let home:AddrData = AddrData{
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

    let mut stream_generic = TcpStream::connect(socket_address[0]);

    // IDK what this does.
    stream_generic.write(&[1])?;
    stream_generic?.read(&mut [0, 128])?;

Ok(())
}

//fn handle_client(stream: TcpStream){
    // .....}

// FN Helper to help create socket_address
fn socket_addr_create(address: (u8,u8,u8,u8), port: u16) -> SocketAddr {
    SocketAddr::from((Ipv4Addr::new(address.0,address.1,address.2,address.3), port))
}
  