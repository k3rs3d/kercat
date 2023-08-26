use async_std::{channel, task, sync::Mutex, io::{self, prelude::*}};
use futures::future::FutureExt;
use log::{error, info};
use std::sync::Arc;

use crate::connection::Connection;
use crate::errors::*;
use crate::Config;

// Asynchronous task that handles sending & receiving data over the network
async fn network_task(
    connection: Arc<Mutex<Connection>>, // Shared state for the connection
    input_receiver: channel::Receiver<String>, // Receiver end for user inputs
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
                        match connection.send_data(input.as_bytes()).await {
                            Ok(_) => info!("User input sent to remote connection"),
                            Err(e) => error!("Error sending data: {:?}", e),
                        }
                    },
                    Err(_) => error!("Error receiving from channel")
                }
            },

            // Wait for a message from the network
            received_result = async {
                let mut connection_lock = connection.lock().await; // Lock
                connection_lock.receive_data().await
            }.fuse() => {
                match received_result {
                    Ok(data) => {
                        println!("{}", data); // Display received data immediately
                        info!("Received data: {}", data);
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

// Entry function; spawns required async tasks
pub async fn start_session(config: &Config) -> SessionResult<()> {
    info!("Starting session with configuration: {:?}", config);

    // Creating a connection using the provided configuration
    let connection = if config.listen {
        Connection::listen(config).await?
    } else {
        Connection::from_config(config).await?
    };
    let connection = Arc::new(Mutex::new(connection));

    info!("Connection created successfully.");

    // Create a channel to communicate between input &  sending tasks
    let (input_sender, input_receiver) = channel::unbounded();

    // Spawn a task to handle user input
    let input_task = task::spawn(async move {
        loop {
            // Prompt for user input - remove?
            print!("> ");
            io::stdout().flush().await?;

            // Read user input from stdin
            let mut input = String::new();
            io::stdin().read_line(&mut input).await?;

            // Send the user input to the network_task
            input_sender.send(input).await?;
        }
        Ok::<(), SessionError>(())
    });

    // Spawn a task to handle network communication (both sending and receiving)
    let network_handle = task::spawn(network_task(
        connection,
        input_receiver,
    ));

    // Await both the user input and network tasks to complete
    // HACK: Makes warning go away
    // TODO: Close the connection with connection.close()
    let network_result = futures::try_join!(network_handle, input_task);
    //let _ = input_task.await?;

    // Check for an exit signal from the network task
    match network_result {
        Ok(_) => info!("Session ended successfully."),
        Err(e) => {
            error!("Session terminated due to: {:?}", e);
            return Err(e);
        },
    }

    info!("Session ended successfully.");
    Ok(())
}
