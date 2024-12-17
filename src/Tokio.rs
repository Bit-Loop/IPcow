use tokio::net::TcpListener;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

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

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let system_thread_count = available_parallelism().unwrap().get(); // Get Thread Count for the current system
    let listener = TcpListener::bind("127.0.0.1:8080").await?;

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


    loop {
        let (mut socket, _) = listener.accept().await?;

        tokio::spawn(async move {
            let mut buf = [0; 1024];

            // In a loop, read data from the socket and write the data back.
            loop {
                let n = match socket.read(&mut buf).await {
                    // socket closed
                    Ok(n) if n == 0 => return,
                    Ok(n) => n,
                    Err(e) => {
                        eprintln!("failed to read from socket; err = {:?}", e);
                        return;
                    }
                };

                // Write the data back
                if let Err(e) = socket.write_all(&buf[0..n]).await {
                    eprintln!("failed to write to socket; err = {:?}", e);
                    return;
                }
            }
        });
    }
}



#[derive(Debug, PartialEq)]
enum AddrType{
///Should consider a different naming scheme
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

// FN Helper to help create socket_address
fn socket_addr_create(address: (u8,u8,u8,u8), port: u16) -> SocketAddr {
    SocketAddr::from((Ipv4Addr::new(address.0,address.1,address.2,address.3), port))
}
  