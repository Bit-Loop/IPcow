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
 * 🚀 Version:   0.2.0
 * 🧑‍💻 Author:       Isaiah Tyler Jackson   
 * 
 *    Todo: Add parsing for IP and port lists!
 * 
 *********************************************************
 */

 use std::io;
 use std::net::Ipv4Addr;
 use ipnetwork::Ipv4Network;
 
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
 /// Supported formats:
 /// - IP range: "192.168.1.1-192.168.1.255"
 /// - CIDR block: "192.168.1.0/24"
 /// - Wildcards: "192.168.X.X" or "X.X.X.X"
 /// - Single IP: "192.168.1.1"
 pub fn parse_ip_input(input: &str) -> Vec<Ipv4Addr> {
    let mut results = Vec::new();

    // Normalize input to uppercase for wildcard processing
    let normalized_input = input.to_uppercase();
    // Initialize Chunk Stack

    if normalized_input.contains('-') {
        // Handle IP range: "192.168.1.1-192.168.1.255"
        let parts: Vec<&str> = normalized_input.split('-').collect();
        if parts.len() == 2 {
            let start: Ipv4Addr = parts[0].parse().expect("Invalid start IP");
            let end: Ipv4Addr = parts[1].parse().expect("Invalid end IP");

            let start_u32 = u32::from(start);
            let end_u32 = u32::from(end);

            if start_u32 > end_u32 {
                panic!("Start IP must be less than or equal to End IP");
            }

            for ip_int in start_u32..=end_u32 {
                results.push(Ipv4Addr::from(ip_int));
            }
        }
    } else if normalized_input.contains('/') {
        // Handle CIDR notation: "192.168.1.0/24"
        let cidr: Ipv4Network = normalized_input.parse().expect("Invalid CIDR format");
        results.extend(cidr.iter());
    } else if normalized_input.contains('X') {
        // Handle wildcard notation: "X.X.X.X" or specific octet wildcards like "192.168.X.X"
        let octets: Vec<&str> = normalized_input.split('.').collect();
        if octets.len() != 4 {
            panic!("Invalid wildcard IP format. Must be like X.X.X.X or similar.");
        }

        let mut ranges = vec![];

        for octet in &octets {
            if *octet == "X" {
                ranges.push(0..=255); // Add full range for wildcard octet
            } else {
                let value: u8 = octet.parse().expect("Invalid octet value");
                ranges.push(value..=value); // Fixed value for non-wildcard octet
            }
        }

        // Iterate over valid IP combinations
        for a in ranges[0].clone() {
            for b in ranges[1].clone() {
                for c in ranges[2].clone() {
                    for d in ranges[3].clone() {
                        let ip = format!("{}.{}.{}.{}", a, b, c, d);
                        if let Ok(parsed_ip) = ip.parse::<Ipv4Addr>() {
                            // Skip IPs ending with .0 unless explicitly specified
                            if !parsed_ip.to_string().ends_with(".0") || input.contains(&parsed_ip.to_string()) {
                                results.push(parsed_ip);
                            }
                        }
                    }
                }
            }
        }
    } else {
        // Single IP address
        if let Ok(ip) = normalized_input.parse::<Ipv4Addr>() {
            results.push(ip);
        }
    }

    results
}

 
 /// Parses port input into a list of ports
 /// Supported formats:
 /// - Port range: "0-65535"
 /// - Comma-separated list: "80, 443, 8080"
 /// - Single port: "8080"
 pub fn parse_port_input(input: &str) -> Vec<u16> {
     let mut ports = Vec::new();
     if input.contains('-') {
         // Handle range: "0-65535"
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
 pub fn addr_input() -> (Vec<Ipv4Addr>, Vec<u16>) {
     // Read IP address input
     let ip_input = read_input(
         "Enter the listen IP addresses.\nFormat: 255.255.255.0-255.255.255.255, 192.168.1.X, or 192.168.1.0/24:",
     );
     // Read port input
     let port_input = read_input(
         "Enter the listen IP ports.\nFormat: 0-65535, or \"1, 2, 5\":",
     );
     // Parse inputs
     let ips = parse_ip_input(&ip_input);
     let ports = parse_port_input(&port_input);
 
     // Output results
     println!("Parsed IP Addresses: {:?}", ips.len());
     println!("Parsed Ports: {:?}", ports.len());
 
     (ips, ports)
 }
