use crate::errors::{SessionError, SessionResult};
use crate::Config;
use async_std::{
    io::prelude::*,
    net::{TcpListener, TcpStream},
};
use std::sync::Arc;
use log::{error, info};

pub struct Connection {
    stream: TcpStream,
    config: Arc<Config>,
}

impl Connection {
    pub async fn from_config(config: Arc<Config>) -> SessionResult<Self> {
        if config.listen {
            Self::listen(config).await
        } else {
            let address = format!("{}:{}", config.host, config.port);
            info!("Connecting to {}", address);
            let stream = TcpStream::connect(&address)
                .await
                .map_err(SessionError::from)?;
            info!("Connected to {}", address);
            stream.set_nodelay(true).map_err(SessionError::from)?;

            Ok(Self { stream, config }) 
        }
    }

    pub async fn listen(config: Arc<Config>) -> SessionResult<Self> {
        let address = format!("{}:{}", config.host, config.port);
        info!("Listening on {}", address);
        let listener = TcpListener::bind(&address)
            .await
            .map_err(SessionError::from)?;
        
        let (stream, addr) = listener.accept().await.map_err(SessionError::from)?;
        info!("Accepted connection from {}", addr);
        stream.set_nodelay(true).map_err(SessionError::from)?;
        Ok(Connection { stream, config })
    }    

    pub async fn receive_data(&mut self) -> SessionResult<Vec<u8>> {
        info!("Receiving data...");
        let mut buffer = vec![0u8; self.config.input_buffer_size];
        let mut total_data = Vec::new();
    
        loop {
            let bytes_read = self.stream.read(&mut buffer).await.map_err(SessionError::from)?;
            
            if bytes_read == 0 {
                error!("Connection closed by the peer");
                return Err(SessionError::Custom("Connection closed by peer".into()));
            }
            
            total_data.extend_from_slice(&buffer[..bytes_read]);
            
            // Check for message boundary, in this case, a newline character.
            if let Some(pos) = total_data.iter().position(|&b| b == b'\n') {
                let received = std::str::from_utf8(&total_data[..pos])
                    .map_err(|e| SessionError::Custom(e.to_string()))?;
                return Ok(total_data);
            }
        }
    }
    
    pub async fn send_data(&mut self, data: &[u8]) -> SessionResult<()> {
        info!("Sending data...");
        self.stream
            .write_all(data)
            .await
            .map_err(SessionError::from)?;
        info!("Data written to stream");
        self.stream.flush().await.map_err(SessionError::from)?;
        info!("Stream flushed");
        Ok(())
    }

    pub async fn close(&mut self) -> SessionResult<()> {
        info!("Closing connection...");
        self.stream
            .shutdown(std::net::Shutdown::Both)
            .map_err(SessionError::from)?;
        Ok(())
    }
}
