use log::{info, error};
use async_std::net::{TcpStream, TcpListener};
use async_std::io::prelude::*;
use async_std::task;
use std::sync::Arc;
use std::error::Error;

use crate::Config;


pub struct Connection {
    stream: TcpStream,
}

impl Connection {
    pub async fn from_config(config: &Config) -> Result<Self, Box<dyn Error>> {
        let config_arc = Arc::new(config.clone()); // Create an Arc from the config
        if config.listen {
            Self::listen(config_arc).await // Pass the Arc to listen
        } else {
            let address = format!("{}:{}", config.host, config.port);
            info!("Connecting to {}", address);
            let stream = TcpStream::connect(&address).await?;
            info!("Connected to {}", address);
            stream.set_nodelay(true)?;

            Ok(Self { stream })
        }
    }

    pub async fn listen(config: Arc<Config>) -> Result<Self, Box<dyn Error>> {
        let address = format!("{}:{}", config.host, config.port);
        info!("Listening on {}", address);
        let listener = TcpListener::bind(&address).await?;

        loop {
            let (stream, addr) = listener.accept().await?;
            info!("Accepted connection from {}", addr);
            stream.set_nodelay(true)?;

            let config_ref = config.clone(); // Clones the Arc, not the Config
        task::spawn(async move {
            let mut connection = Connection { stream };
            connection.handle_listening(&*config_ref).await; // Dereferences the Arc
        });
    }

    }

    pub async fn handle_listening(&mut self, _config: &Config) {
        loop {
            match self.receive_data().await {
                Ok(data) => {
                    // TODO: Handle received data here etc
                }
                Err(e) => {
                    error!("Error receiving data: {}", e);
                    break;
                }
            }
        }
    }

    pub async fn receive_data(&mut self) -> Result<String, Box<dyn Error + Send>> {
        info!("Receiving data...");
        let mut buffer = [0u8; 1024];
        let bytes_read = self.stream.read(&mut buffer).await.map_err(|e| -> Box<dyn Error + Send> {
            Box::new(e)
        })?;
        if bytes_read == 0 {
            // Connection closed by peer
            error!("Connection closed by the peer");
            return Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::ConnectionAborted,
                "Connection closed by peer",
            )));
        }
        let received = std::str::from_utf8(&buffer[..bytes_read]).map_err(|e| -> Box<dyn Error + Send> {
            Box::new(e)
        })?;
        info!("Received: {}", received);
        Ok(received.to_string())
    }
    

    pub async fn send_data(&mut self, data: &[u8]) -> Result<(), Box<dyn Error>> {
        info!("Sending data...");
        self.stream.write_all(data).await?;
        info!("Data written to stream");
        self.stream.flush().await?;
        info!("Stream flushed");
        Ok(())
    }

    pub async fn close(&mut self) -> Result<(), Box<dyn Error>> {
        info!("Closing connection...");
        self.stream.shutdown(std::net::Shutdown::Both)?;
        Ok(())
    }
}