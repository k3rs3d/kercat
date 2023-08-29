use async_std::{
    channel,
    io::{self, prelude::*},
    task,
};
use futures::future::{Fuse, FutureExt};
use log::{debug, error, info};
use std::{pin::Pin, sync::Arc};

use crate::connection::Connection;
use crate::errors::*;
use crate::Config;

// Entry function; spawns required async tasks
pub async fn start_session(config: Arc<Config>) -> SessionResult<()> {
    info!("Starting session with configuration: {:?}", config);

    // Create a channel to communicate between input &  sending tasks
    let (input_sender, input_receiver) = channel::unbounded::<Vec<u8>>();
    task::spawn(input_task(input_sender, config.clone()));

    // For storing the result of a network task.
    let mut network_handle: Option<Pin<Box<Fuse<_>>>> = None;

    // Loop through each address to establish a connection
    for socket_address in &config.addresses {
        // If there's a previous handle, await its result (and log if failed)
        if let Some(handle) = network_handle.take() {
            let result: Result<(), SessionError> = handle.await;
            if result.is_err() {
                error!("Connection to {} failed, proceeding to next address.", socket_address);
            }
        }        

        // Try to establish a connection 
        match establish_connection(&config, socket_address).await {
            Ok(connection) => {
                // If successful, spawn a new network task
                network_handle = Some(Box::pin(
                    task::spawn(network_task(connection, input_receiver.clone())).fuse(),
                ));
            }
            // If connection fails, proceed to the next address.
            Err(_) => continue,
        }
    }

    // If there's a remaining network handle, await its result.
    if let Some(handle) = network_handle {
        handle.await?;
    }

    Ok(())
}

// Helper function; attempts to create a new Connection from the address
async fn establish_connection(
    config: &Arc<Config>,
    address: &std::net::SocketAddr,
) -> Result<Arc<Connection>, SessionError> {
    match Connection::from_config(config.clone(), *address).await {
        Ok(connection) => Ok(Arc::new(connection)),
        Err(e) => {
            error!(
                "Failed to initialize connection to {}: {:?}, proceeding to next address.",
                address, e
            );
            Err(e.into())
        }
    }
}

// Asynchronous task that handles sending & receiving data over the network
async fn network_task(
    connection: Arc<Connection>, // Shared state for the connection
    input_receiver: channel::Receiver<Vec<u8>>, // Receiver end for user inputs
) -> SessionResult<()> {
    loop {
        // Concurrently await input & network
        futures::select! {
            // Wait for a message from the user to send over the network
            input_result = input_receiver.recv().fuse() => {
                match input_result {
                    Ok(input) => {
                        info!("Received input from user, attempting to send data.");
                        // Send user input to the remote connection
                        match connection.send_data(&input).await {
                            Ok(_) => info!("User input sent to remote connection"),
                            Err(e) => error!("Error sending data: {:?}", e),
                        }
                    },
                    Err(_) => {
                        info!("Input channel has been closed; terminating connection.");
                        connection.close().await?; // Close the connection
                        return Ok(());  // Terminate the loop, ending the network_task
                    },
                }
            },

            // Wait for a message from the network
            received_result = async {
                connection.receive_data().await
            }.fuse() => {
                match received_result {
                    Ok(data) => {
                        println!("{:?}", data); // Display received data immediately
                        info!("Received data: {:?}", data);
                    },
                    Err(e) => {
                        error!("Error receiving data: {:?}", e);
                        return Err(e.into());  // Propagate error back
                    },
                }
            },
        }
    }
}

async fn input_task(
    input_sender: channel::Sender<Vec<u8>>,
    config: Arc<Config>,
) -> SessionResult<()> {
    loop {
        // Prompt for user input - remove?
        print!("> ");
        io::stdout().flush().await?;

        // Read user input from stdin
        let mut input = vec![0u8; 1024]; // TODO: configurable buffer size
        let bytes_read = io::stdin().read(&mut input).await?;

        // Check for EoF
        if bytes_read == 0 {
            if config.ignore_eof {
                // used cloned config
                debug!(
                    "EOF detected on stdin but ignoring due to config: {:?}",
                    config
                );
                continue;
            }
            info!("EOF detected on stdin. Closing connection...");
            break;
        }

        input.truncate(bytes_read); // Truncate buffer to actual size
        input_sender.send(input).await?; // Send input to network_task
    }

    Ok::<(), SessionError>(())
}