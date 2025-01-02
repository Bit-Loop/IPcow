pub mod test_utils {
    use std::net::TcpStream;
    use std::io::Write;
    use std::time::Duration;

    pub fn send_test_data(addr: &str, data: &[u8]) -> std::io::Result<()> {
        let mut stream = TcpStream::connect(addr)?;
        stream.set_write_timeout(Some(Duration::from_secs(5)))?;
        stream.write_all(data)?;
        Ok(())
    }
}