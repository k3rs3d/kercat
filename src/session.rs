use async_std::{
    channel,
    io::{self, prelude::*},
    task,
};
use futures::{StreamExt, future::{Fuse, FutureExt}};
use log::{debug, error, info};
use std::{pin::Pin, sync::Arc};

use crate::connection::Connection;
use crate::errors::*;
use crate::Config;
pub enum SessionEvent {
    Input(Vec<u8>),
    NetworkData(Vec<u8>),
    Error(SessionError),
    ConnectionClose,
}

// Entry function; spawns required async tasks
pub async fn start_session(config: Arc<Config>) -> SessionResult<()> {
    info!("Starting session with configuration: {:?}", config);

    // Create a channel to communicate between input &  sending tasks
    let (event_sender, event_receiver) = channel::unbounded::<SessionEvent>();
    task::spawn(input_task(event_sender.clone(), config.clone()));

    // For storing the result of a network task
    let mut network_handle: Option<Pin<Box<Fuse<_>>>> = None;

    let mut event_loop = event_receiver.fuse();

    // Loop through each address to establish a connection
    for socket_address in &config.addresses {
        if let Some(handle) = network_handle.take() {
            let result: Result<(), SessionError> = handle.await;
            if result.is_err() {
                event_sender.send(SessionEvent::Error(result.unwrap_err())).await?;
            }
        }

        match establish_connection(&config, socket_address).await {
            Ok(connection) => {
                network_handle = Some(Box::pin(
                    task::spawn(network_task(event_sender.clone(), connection)).fuse(),
                ));
            }
            Err(_) => continue,
        }
    }

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

// Asynchronous task that handles sending & receiving network events
async fn network_task(
    event_sender: channel::Sender<SessionEvent>,
    connection: Arc<Connection>,
) -> SessionResult<()> {
    loop {
        futures::select! {
            received_result = async {
                connection.receive_data().await
            }.fuse() => {
                match received_result {
                    Ok(data) => {
                        event_sender.send(SessionEvent::NetworkData(data)).await?;
                    },
                    Err(e) => {
                        event_sender.send(SessionEvent::Error(e)).await?;
                    },
                }
            },
        }
    }
}

async fn input_task(
    event_sender: channel::Sender<SessionEvent>,
    config: Arc<Config>,
) -> SessionResult<()> {
    loop {
        print!("> ");
        io::stdout().flush().await?;

        let mut input = vec![0u8; config.input_buffer_size];
        let bytes_read = io::stdin().read(&mut input).await?;

        if bytes_read == 0 {
            if config.ignore_eof {
                debug!(
                    "EOF detected on stdin but ignoring due to config: {:?}",
                    config
                );
                continue;
            }
            event_sender.send(SessionEvent::ConnectionClose).await?;
            break;
        }

        input.truncate(bytes_read); // Truncate buffer to actual size
        event_sender.send(SessionEvent::Input(input)).await?; // Send input
    }

    Ok(())
}