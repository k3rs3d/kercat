use log::{error, info};
use async_std::io;
use async_std::net::TcpStream;
use async_std::prelude::*;

pub async fn start_client(host: &str, port: &str) -> Result<(), Box<dyn std::error::Error>> {
    let address = format!("{}:{}", host, port);
    info!("Connecting to {}:{}", host, port);

    let mut stream = TcpStream::connect(&address).await?;
    stream.set_nodelay(true)?;

    info!("Connected successfully!");

    loop {
        let mut input = String::new();
        print!("> ");
        io::stdout().flush().await?;
        io::stdin().read_line(&mut input).await?;
        info!("Read input from user: {}", input.trim());

        stream.write_all(input.as_bytes()).await?;
        stream.flush().await?;

        let mut buffer = [0u8; 1024];
        let bytes_read = stream.read(&mut buffer).await?;

        if bytes_read == 0 {
            break;
        }

        let received = std::str::from_utf8(&buffer[..bytes_read])?;
        println!("Server: {}", received);
        info!("Received data from server: {}", received);
    }

    Ok(())
}
