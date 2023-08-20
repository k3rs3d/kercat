use std::net::TcpListener;
use std::io::Read;
use log::{info,debug};

pub fn start_server(port: &str) -> Result<(), Box<dyn std::error::Error>> {
    let listener = TcpListener::bind(format!("0.0.0.0:{}", port))?;
    info!("Server started, listening on port {}", port);

    // Accept connections and process them serially
    for stream in listener.incoming() {
        let mut stream = stream?;
        info!("Connection accepted from {}", stream.peer_addr()?);

        let mut buffer = [0; 1024];
        loop {
            let bytes_read = stream.read(&mut buffer)?;
            if bytes_read == 0 {
                // Connection closed
                info!("Connection closed from {}", stream.peer_addr()?);
                break;
            }

            debug!("Received {} bytes from {}", bytes_read, stream.peer_addr()?);
            print!("{}", String::from_utf8_lossy(&buffer[..bytes_read]));
        }
    }

    Ok(())
}