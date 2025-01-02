/* Required dependencies and requires root access
 * 
 * [dependencies]
 * nix = "0.26"
 * libc = "0.2"
 * 
 * Requires root access to run
 */

[dependencies]
nix = "0.26"
libc = "0.2" */


use nix::sys::socket::{
    socket, 
    AddressFamily, 
    SockFlag, 
    SockType,
    bind,
    sendto,
    recv
};
use nix::sys::socket::LinkAddr;
use libc::{AF_PACKET, SOCK_RAW};
use std::net::NetworkInterface;

struct RawInterface {
    sock_fd: i32,
    interface: String,
    if_index: i32
}

impl RawInterface {
    fn new(interface_name: &str) -> Result<Self, Box<dyn std::error::Error>> {
        // Create raw socket
        let sock_fd = unsafe {
            socket(
                AF_PACKET,
                SOCK_RAW,
                (libc::ETH_P_ALL as u16).to_be() as i32,
            )?
        };

        // Get interface index
        let if_index = unsafe {
            libc::if_nametoindex(interface_name.as_ptr() as *const i8)
        } as i32;

        Ok(Self {
            sock_fd,
            interface: interface_name.to_string(),
            if_index
        })
    }

    fn send_raw(&self, data: &[u8]) -> Result<usize, Box<dyn std::error::Error>> {
        let addr = LinkAddr::new(self.if_index, libc::ETH_P_ALL);
        Ok(sendto(self.sock_fd, data, &addr, SockFlag::empty())?)
    }

    fn recv_raw(&self, buffer: &mut [u8]) -> Result<usize, Box<dyn std::error::Error>> {
        Ok(recv(self.sock_fd, buffer, SockFlag::empty())?)
    }
}

// Usage example:
async fn raw_interface_example() -> Result<(), Box<dyn std::error::Error>> {
    let interface = RawInterface::new("eth0")?;
    
    let mut recv_buffer = [0u8; 1500]; // MTU size buffer
    
    loop {
        let bytes_read = interface.recv_raw(&mut recv_buffer)?;
        println!("Received {} bytes", bytes_read);
        
        // Process raw packet data here
    }
}