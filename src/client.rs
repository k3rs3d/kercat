use log::{error, info};
use async_std::io;
use async_std::net::TcpStream;
use async_std::prelude::*;

pub async fn start_client(host: &str, port: &str) -> Result<(), Box<dyn std::error::Error>> {
    let address = format!("{}:{}", host, port);

    // Log the connection initiation
    info!("Connecting to {}:{}", host, port);

    // Connect to the server asynchronously and log an error if it fails
    let mut stream = TcpStream::connect(&address)
        .await
        .map_err(|e| {
            let msg = format!("Failed to connect to {}: {}.", address, e);
            error!("{}", msg);
            msg
        })?;

    // Set no delay to true on the connection to avoid latency issues
    stream
        .set_nodelay(true)
        .map_err(|e| {
            let msg = format!("Failed to set nodelay on the connection to {}: {}", address, e);
            error!("{}", msg);
            msg
        })?;

    // Log successful connection
    info!("Connected successfully!");

    // Main loop to handle user input and communication with the server
    loop {
        let mut input = String::new();
        print!("> ");

        // Flush stdout asynchronously to ensure the user prompt is displayed
        io::stdout()
            .flush()
            .await
            .map_err(|e| {
                let msg = format!("Failed to flush stdout: {}", e);
                error!("{}", msg);
                msg
            })?;

        // Read user input asynchronously, and log an error if it fails
        io::stdin()
            .read_line(&mut input)
            .await
            .map_err(|e| {
                let msg = format!("Failed to read line from stdin: {}", e);
                error!("{}", msg);
                msg
            })?;

        // Log the received input
        info!("Read input from user: {}", input.trim());

        // Send the user input to the server asynchronously
        stream
            .write_all(input.as_bytes())
            .await
            .map_err(|e| {
                let msg = format!("Failed to write to {}: {}", address, e);
                error!("{}", msg);
                msg
            })?;

        // Flush the stream to ensure the data is sent
        stream
            .flush()
            .await
            .map_err(|e| {
                let msg = format!("Failed to flush to {}: {}", address, e);
                error!("{}", msg);
                msg
            })?;

        let mut buffer = [0u8; 1024];

        // Read the response from the server asynchronously
        let bytes_read = stream
            .read(&mut buffer)
            .await
            .map_err(|e| {
                let msg = format!("Failed to read from {}: {}", address, e);
                error!("{}", msg);
                msg
            })?;

        // Check if the connection was closed by the server
        if bytes_read == 0 {
            info!("Connection closed by server.");
            break;
        }

        // Convert the received bytes to a string and print it to the console
        let received = std::str::from_utf8(&buffer[..bytes_read])
            .map_err(|e| {
                let msg = format!("Failed to convert received bytes to string: {}", e);
                error!("{}", msg);
                msg
            })?;
        println!("Server: {}", received);

        // Log the data received from the server
        info!("Received data from server: {}", received);
    }

    Ok(())
}
