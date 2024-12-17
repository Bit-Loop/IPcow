/*!
 *********************************************************
 *                     🔍 SockParse 🔍                  
 *      A lightweight Rust library for parsing user  
 *      inputs into socket addresses for TCP/UDP.    
 * -----------------------------------------------------
 * ✨ Features:
 *   - Parse IP ranges, CIDR blocks, and wildcards.
 *   - Handle port ranges and lists with ease.
 *   - Output ready-to-use `SocketAddr` arrays for Tokio.
 *
 * 🚀 Version:   0.1.0  
 * 🛠️ Author:    Isaiah Tyler Jackson  
 *********************************************************
 */
use std::io;
use std::net::Ipv4Addr;

/// Reads input from user with a prompt
fn read_input(prompt: &str) -> String {
    let mut input = String::new();
    println!("{}", prompt);
    io::stdin()
        .read_line(&mut input)
        .expect("Failed to read input.");
    input.trim().to_string()
}

/// Parses IP address input into supported formats
fn parse_ip_input(input: &str) -> Vec<String> {
    let mut results = Vec::new();

    if input.contains('-') {
        // Handle IP range: "192.168.1.1-192.168.1.255"
        let parts: Vec<&str> = input.split('-').collect();
        if parts.len() == 2 {
            let start: Ipv4Addr = parts[0].parse().expect("Invalid start IP");
            let end: Ipv4Addr = parts[1].parse().expect("Invalid end IP");

            let mut current = start.octets();
            let end_octets = end.octets();

            while current <= end_octets {
                results.push(format!("{}.{}.{}.{}", current[0], current[1], current[2], current[3]));
                current[3] += 1;
                if current[3] == 0 {
                    current[2] += 1;
                }
            }
        }
    } else if input.contains('/') {
        // Handle CIDR notation: "192.168.1.0/24"
        let cidr: ipnetwork::Ipv4Network = input
            .parse()
            .expect("Invalid CIDR format");
        results.extend(cidr.iter().map(|ip| ip.to_string()));
    } else if input.contains('X') {
        // Handle wildcard notation: "192.168.1.X"
        for i in 0..=255 {
            results.push(input.replace('X', &i.to_string()));
        }
    } else {
        results.push(input.to_string()); // Single IP address
    }

    results
}

/// Parses port input into a list of ports
fn parse_port_input(input: &str) -> Vec<u16> {
    let mut ports = Vec::new();

    if input.contains('-') {
        // Handle range: "0-65536"
        let parts: Vec<&str> = input.split('-').collect();
        if parts.len() == 2 {
            let start: u16 = parts[0].parse().expect("Invalid start port");
            let end: u16 = parts[1].parse().expect("Invalid end port");
            for port in start..=end {
                ports.push(port);
            }
        }
    } else if input.contains(',') {
        // Handle list of ports: "1, 2, 5"
        for p in input.split(',') {
            let port: u16 = p.trim().parse().expect("Invalid port number");
            ports.push(port);
        }
    } else {
        // Single port
        ports.push(input.parse().expect("Invalid port"));
    }

    ports
}

/// Main function for input and parsing
fn addr_input() -> (Vec<String>, Vec<u16>) {
    // Read IP address input
    let ip_input = read_input(
        "Enter the listen IP addresses.\nFormat: 255.255.255.0-255.255.255.255, 192.168.1.X, or 192.168.1.0/24:",
    );

    // Read port input
    let port_input = read_input(
        "Enter the listen IP ports.\nFormat: 0-65536, or \"1, 2, 5\":",
    );

    // Parse inputs
    let ips = parse_ip_input(&ip_input);
    let ports = parse_port_input(&port_input);

    // Output results
    println!("Parsed IP Addresses: {:?}", ips);
    println!("Parsed Ports: {:?}", ports);

    (ips, ports)
}

fn main() {
    let (ips, ports) = addr_input();
    println!("\nFinal IPs: {:?}", ips);
    println!("Final Ports: {:?}", ports);
}
