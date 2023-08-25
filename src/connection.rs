use log::{info, error};
use async_std::net::{TcpStream, TcpListener};
use async_std::io::prelude::*;
use async_std::task;
use std::sync::Arc;

use crate::Config;
use crate::errors::SessionError;

pub struct Connection {
    stream: TcpStream,
}

impl Connection {
    pub async fn from_config(config: &Config) -> Result<Self, SessionError> { // Replace Box<dyn Error> with SessionError
        let config_arc = Arc::new(config.clone());
        if config.listen {
            Self::listen(config_arc).await
        } else {
            let address = format!("{}:{}", config.host, config.port);
            info!("Connecting to {}", address);
            let stream = TcpStream::connect(&address).await.map_err(SessionError::from)?;
            info!("Connected to {}", address);
            stream.set_nodelay(true).map_err(SessionError::from)?;

            Ok(Self { stream })
        }
    }

    pub async fn listen(config: Arc<Config>) -> Result<Self, SessionError> {
        let address = format!("{}:{}", config.host, config.port);
        info!("Listening on {}", address);
        let listener = TcpListener::bind(&address).await.map_err(SessionError::from)?;

        loop {
            let (stream, addr) = listener.accept().await.map_err(SessionError::from)?;
            info!("Accepted connection from {}", addr);
            stream.set_nodelay(true).map_err(SessionError::from)?;

            let config_ref = config.clone();
            task::spawn(async move {
                let mut connection = Connection { stream };
                connection.handle_listening(&*config_ref).await;
            });
        }
    }

    pub async fn handle_listening(&mut self, _config: &Config) {
        loop {
            match self.receive_data().await {
                Ok(data) => {
                    // TODO: Handle received data here
                }
                Err(e) => {
                    error!("Error receiving data: {}", e);
                    break;
                }
            }
        }
    }

    pub async fn receive_data(&mut self) -> Result<String, SessionError> {
        info!("Receiving data...");
        let mut buffer = [0u8; 1024];
        let bytes_read = self.stream.read(&mut buffer).await.map_err(SessionError::from)?;

        if bytes_read == 0 {
            error!("Connection closed by the peer");
            return Err(SessionError::Custom("Connection closed by peer".into()));
        }

        let received = std::str::from_utf8(&buffer[..bytes_read]).map_err(|e| SessionError::Custom(e.to_string()))?;
        info!("Received: {}", received);
        Ok(received.to_string())
    }

    pub async fn send_data(&mut self, data: &[u8]) -> Result<(), SessionError> {
        info!("Sending data...");
        self.stream.write_all(data).await.map_err(SessionError::from)?;
        info!("Data written to stream");
        self.stream.flush().await.map_err(SessionError::from)?;
        info!("Stream flushed");
        Ok(())
    }

    pub async fn close(&mut self) -> Result<(), SessionError> {
        info!("Closing connection...");
        self.stream.shutdown(std::net::Shutdown::Both).map_err(SessionError::from)?;
        Ok(())
    }
}