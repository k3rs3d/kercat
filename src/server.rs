use async_std::io;
use async_std::net::TcpListener;
use async_std::prelude::*;
use log::info;
use crate::Config;

pub async fn start_server(config: &Config) -> Result<(), Box<dyn std::error::Error>> {
    let listener = TcpListener::bind(format!("0.0.0.0:{}", config.port)).await?;
    info!("Server started, listening on port {}", config.port);

    // Accept connections and process them concurrently
    let mut incoming = listener.incoming();
    while let Some(stream) = incoming.next().await {
        let mut stream = stream?;
        info!("Connection accepted from {}", stream.peer_addr()?);

        let mut buffer = [0; 1024];
        loop {
            let bytes_read = stream.read(&mut buffer).await?;
            if bytes_read == 0 {
                // Connection closed
                info!("Connection closed from {}", stream.peer_addr()?);
                break;
            }

            info!("Received {} bytes from {}", bytes_read, stream.peer_addr()?);
            print!("{}", String::from_utf8_lossy(&buffer[..bytes_read]));
            io::stdout().flush().await?; // Flush the buffer immediately
        }
    }

    Ok(())
}
