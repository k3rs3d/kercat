use crate::connection::Connection;
use crate::Config;
use async_std::channel;
use async_std::io::{self, prelude::*};
use async_std::sync::Mutex;
use async_std::task;
use log::{error, info};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use futures::future::FutureExt;

async fn network_task(
    connection: Arc<Mutex<Connection>>,
    input_receiver: channel::Receiver<String>,
    should_exit: Arc<AtomicBool>,
) -> Result<(), String> {
    loop {
        // Concurrently await input and network reception.
        futures::select! {
            input_result = input_receiver.recv().fuse() => {
                if let Ok(input) = input_result {
                    info!("Received input from user, attempting to send data.");
                    let mut connection = connection.lock().await;
                    match connection.send_data(input.as_bytes()).await {
                        Ok(_) => info!("User input sent to remote connection"),
                        Err(e) => error!("Error sending data: {:?}", e),
                    };
                }
            },
            received_result = async {
                let mut connection_lock = connection.lock().await;
                connection_lock.receive_data().await
            }.fuse() => {
                match received_result {
                    Ok(data) => {
                        println!("{}", data); // Display received message
                        info!("Received data: {}", data); // Logging received data
                    },
                    Err(e) => {
                        if let Some(io_err) = e.downcast_ref::<std::io::Error>() {
                            if io_err.kind() == std::io::ErrorKind::ConnectionAborted {
                                info!("Connection closed by the server, exiting network task");
                                should_exit.store(true, Ordering::SeqCst); // Signal to exit
                                break;
                            }
                        }
                        error!("Error receiving data: {:?}", e);
                        break;
                    }
                };
            },
        }

        if should_exit.load(Ordering::SeqCst) {
            break;
        }
    }

    Ok(())
}


pub async fn start_session(config: &Config) -> Result<(), String> {
    info!("Starting session with configuration: {:?}", config);

    let should_exit = Arc::new(AtomicBool::new(false));
    let should_exit_for_input = should_exit.clone(); // Clone for the input task

    // Creating a connection using the provided configuration
    let connection = Connection::from_config(config).await.map_err(|e| {
        error!("Error creating connection: {:?}", e);
        format!("{:?}", e)
    })?;
    let connection = Arc::new(Mutex::new(connection));
    info!("Connection created successfully.");

    // Create a channel to communicate between input &  sending tasks
    let (input_sender, input_receiver) = channel::unbounded();

    // Spawn a task to handle user input
    let input_task = task::spawn(async move {
        let should_exit = should_exit_for_input; // Use the cloned value
        loop {
            print!("> ");
            if let Err(e) = io::stdout().flush().await {
                error!("Error flushing stdout: {:?}", e);
            }

            let mut input = String::new();

            if let Err(e) = io::stdin().read_line(&mut input).await {
                error!("Error reading input: {:?}", e);
            } else {
                info!("Received user input: {:?}", input);
                if let Err(e) = input_sender.send(input).await {
                    error!("Error sending user input to network task: {:?}", e);
                }
            }

            // Check for an exit signal from the network task
            if should_exit.load(Ordering::SeqCst) {
                break;
            }
        }
    });

    // Spawn a task to handle network communication (both sending and receiving)
    let should_exit_for_network = should_exit.clone();
    let network_handle = task::spawn(network_task(connection, input_receiver, should_exit_for_network));
    info!("Network task spawned successfully.");

    // Wait for the network task to complete
    let network_result = network_handle.await.map_err(|e| {
        error!("Network task ended with an error: {:?}", e);
        format!("{:?}", e)
    });

    // TODO: Close the connection with connection.close

    input_task.await;
    info!("Session ended successfully.");
    
    if should_exit.load(Ordering::SeqCst) {
        return Err("Connection closed by the server, session terminated.".to_string());
    }
    
    network_result;

    Ok::<(), String>(())
}
