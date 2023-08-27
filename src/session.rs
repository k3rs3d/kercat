use async_std::{
    channel,
    io::{self, prelude::*},
    sync::Mutex,
    task,
};
use futures::future::{Fuse, FutureExt};
use log::{debug, error, info};
use std::{pin::Pin, sync::Arc};

use crate::connection::Connection;
use crate::errors::*;
use crate::Config;

// Asynchronous task that handles sending & receiving data over the network
async fn network_task(
    connection: Arc<Mutex<Connection>>, // Shared state for the connection
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
                        let mut connection = connection.lock().await; // Lock
                        // Send user input to the remote connection
                        match connection.send_data(&input).await {
                            Ok(_) => info!("User input sent to remote connection"),
                            Err(e) => error!("Error sending data: {:?}", e),
                        }
                    },
                    Err(_) => {
                        info!("Input channel has been closed; terminating connection.");
                        let mut connection = connection.lock().await; // Lock
                        connection.close().await?; // Close the connection
                        return Ok(());  // Terminate the loop, ending the network_task
                    },
                }
            },

            // Wait for a message from the network
            received_result = async {
                let mut connection_lock = connection.lock().await; // Lock
                connection_lock.receive_data().await
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

// Entry function; spawns required async tasks
pub async fn start_session(config: Arc<Config>) -> SessionResult<()> {
    info!("Starting session with configuration: {:?}", config);

    // Creating a connection using the provided configuration
    let config_clone = config.clone(); // For the task

    info!("Connection created successfully.");

    // Create a channel to communicate between input &  sending tasks
    let (input_sender, input_receiver) = channel::unbounded::<Vec<u8>>();

    // Spawn a task to handle user input and make it a FusedFuture
    let mut input_task: Pin<Box<Fuse<_>>> =
        Box::pin(task::spawn(input_task(input_sender, config.clone())).fuse());

    let mut network_handle: Option<Pin<Box<Fuse<_>>>> = None;

    for socket_address in &config_clone.addresses {
        let mut temp_handle = network_handle.take(); // Take ownership and set network_handle to None

        if let Some(mut handle) = temp_handle {
            match handle.await {
                Ok(_) => {
                    info!("Connection to {} was successful.", socket_address);
                }
                Err(e) => {
                    error!(
                        "Connection to {} failed with error: {:?}, proceeding to next address.",
                        socket_address, e
                    );
                }
            }
        }

        match Connection::from_config(config_clone.clone(), *socket_address).await {
            Ok(connection) => {
                let connection = Arc::new(Mutex::new(connection));
    
                // Spawn a task to handle network communication (both sending and receiving)
                temp_handle = Some(Box::pin(
                    task::spawn(network_task(connection, input_receiver.clone())).fuse(),
                ));
                network_handle = temp_handle; // Give ownership back to network_handle for the next loop iteration
            }
            Err(e) => {
                error!(
                    "Failed to initialize connection to {}: {:?}, proceeding to next address.",
                    socket_address, e
                );
                continue; // Skip the rest of this iteration and proceed to the next address
            }
        }
    }

    // Wait for the last network task to finish, if there is one
    if let Some(handle) = network_handle {
        handle.await?;
    }

    Ok(())
}
