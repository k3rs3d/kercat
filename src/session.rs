use async_std::channel;
use async_std::io::{self, prelude::*};
use async_std::sync::Mutex;
use async_std::task;
use log::{error, info};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use futures::future::FutureExt;

use crate::errors::{SessionError,SessionResult};
use crate::connection::Connection;
use crate::Config;

// Asynchronous task that handles sending & receiving data over the network 
async fn network_task(
    connection: Arc<Mutex<Connection>>, // Shared state for the connection
    input_receiver: channel::Receiver<String>, // Receiver end for user inputs
    should_exit: Arc<AtomicBool>, // Exit signal to break the loop
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
                    Err(e) => error!("Error receiving data: {:?}", e),
                }
            },
        }

        // Condition for breaking the loop. Checked after every network operation
        if should_exit.load(Ordering::SeqCst) {
            break;
        }
    }

    Ok(())
}

// Entry function; spawns required async tasks
pub async fn start_session(config: &Config) -> SessionResult<()> {
    info!("Starting session with configuration: {:?}", config);

    // Use atomic bool for non-blocking state sharing across async tasks.
    let should_exit = Arc::new(AtomicBool::new(false));
    let should_exit_for_input = should_exit.clone(); // Clone for the input task

    // Creating a connection using the provided configuration
    let connection = Connection::from_config(config).await?;
    let connection = Arc::new(Mutex::new(connection));

    info!("Connection created successfully.");

    // Create a channel to communicate between input &  sending tasks
    let (input_sender, input_receiver) = channel::unbounded();

    // Spawn a task to handle user input
    let input_task = task::spawn(async move {
        loop {
            // Prompt for user input - remove? 
            print!("> ");
            match io::stdout().flush().await {
                Ok(_) => (),
                Err(e) => error!("Error flushing stdout: {:?}", e),
            }

            // Read user input from stdin
            let mut input = String::new();
            match io::stdin().read_line(&mut input).await {
                Ok(_) => info!("Received user input: {:?}", input),
                Err(e) => error!("Error reading line: {:?}", e),
            }

            // Send the user input to the network_task
            match input_sender.send(input).await {
                Ok(_) => (),
                Err(e) => error!("Error sending user input to network task: {:?}", e),
            }

            // Check for an exit signal from the network task
            if should_exit_for_input.load(Ordering::SeqCst) { // Using the clone here
                break;
            }
        }

        Ok::<(), SessionError>(())
    });

    // Spawn a task to handle network communication (both sending and receiving)
    let should_exit_for_network = should_exit.clone(); // Clone for the network task
    let network_handle = task::spawn(network_task(connection, input_receiver, should_exit_for_network));

    // Await both the user input and network tasks to complete
    // HACK: Makes warning go away
    // TODO: Close the connection with connection.close()
    let _ = network_handle.await?;
    let _ = input_task.await?;

    // Check for an exit signal from network task
    if should_exit.load(Ordering::SeqCst) {
        return Err(SessionError::Custom("Connection closed by the server, session terminated.".to_string()));
    }

    info!("Session ended successfully.");
    Ok(())
}
