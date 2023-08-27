use log::{info, error};
use std::sync::Arc;
use async_std::task;

mod args; 
mod session;
mod connection;
mod errors;

#[derive(Debug, Clone)]
pub struct Config {
    addr_type: AddressType,
    addresses: Vec<async_std::net::SocketAddr>,
    listen: bool, 
    keep_listening: bool,
    input_buffer_size: usize,
    output_buffer_size: usize,
    ignore_eof: bool,
    // More in the future...
}

#[derive(Debug, Copy, Clone)]
enum AddressType {
    IP, //v4 OR v6
    IPv4,
    IPv6,
    //UnixDomainSocket,
}


fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Parse command-line arguments to build Configf
    let config = Arc::new(args::parse_args()?);

    // Start the session; block_on is used to run the async function synchronously
    info!("Starting session with config: {:?}", &config);
    let result = task::block_on(session::start_session(config));

    // Handle session result
    match result {
        Ok(_) => info!("Session ended successfully"),
        Err(e) => error!("Error during session: {}", e),
    }

    Ok(())
}
