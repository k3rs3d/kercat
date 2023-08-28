use crate::errors::{SessionError, SessionResult};
use crate::Config;
use async_std::{
    io::prelude::*,
    net::{SocketAddr, TcpListener, TcpStream},
    sync::Mutex,
};
use async_std_resolver::{resolver, config};
use std::sync::Arc;
use log::{error, info};

pub struct Connection {
    stream: Arc<Mutex<TcpStream>>,
    config: Arc<Config>,
}

impl Connection {
    pub async fn from_config(config: Arc<Config>, address: SocketAddr) -> SessionResult<Self> {
        let mut address = address;
        
        if !config.ignore_dns {
            address = Self::resolve_hostname(address).await?;
        }

        if config.listen {
            Self::listen(config, address).await
        } else {
            info!("Connecting to {}", address);
            let stream = TcpStream::connect(&address)
                .await
                .map_err(SessionError::from)?;
            info!("Connected to {}", address);
            stream.set_nodelay(true).map_err(SessionError::from)?;
            let stream = Arc::new(Mutex::new(stream));

            Ok(Self { stream, config }) 
        }
    }

    pub async fn listen(config: Arc<Config>, address: SocketAddr) -> SessionResult<Self> {
        info!("Listening on {}", address);
        let listener = TcpListener::bind(&address)
            .await
            .map_err(SessionError::from)?;
        
        let (stream, addr) = listener.accept().await.map_err(SessionError::from)?;
        info!("Accepted connection from {}", addr);
        stream.set_nodelay(true).map_err(SessionError::from)?;
        let stream = Arc::new(Mutex::new(stream));

        Ok(Connection { stream, config })
    }    

    pub async fn receive_data(&self) -> SessionResult<Vec<u8>> {
        info!("Receiving data...");
        let mut stream = self.stream.lock().await;
        let mut buffer = vec![0u8; self.config.input_buffer_size];
        let mut total_data = Vec::new();
    
        loop {
            let bytes_read = stream.read(&mut buffer).await.map_err(SessionError::from)?;
            
            if bytes_read == 0 {
                error!("Connection closed by the peer");
                return Err(SessionError::ClientDisconnected);
            }
            
            total_data.extend_from_slice(&buffer[..bytes_read]);
            
            if let Some(pos) = total_data.iter().position(|&b| b == b'\n') {
                return Ok(total_data);
            }
        }
    }
    
    pub async fn send_data(&self, data: &[u8]) -> SessionResult<()> {
        info!("Sending data...");
        let mut stream = self.stream.lock().await;
        stream
            .write_all(data)
            .await
            .map_err(SessionError::from)?;
        info!("Data written to stream");
        stream.flush().await.map_err(SessionError::from)?;
        info!("Stream flushed");
        Ok(())
    }

    pub async fn close(&self) -> SessionResult<()> {
        info!("Closing connection...");
        let mut stream = self.stream.lock().await;
        stream
            .shutdown(async_std::net::Shutdown::Both)
            .map_err(SessionError::from)?;
        Ok(())
    }

    async fn resolve_hostname(address: SocketAddr) -> Result<SocketAddr, std::io::Error> {
        if address.ip().is_unspecified() {
            let resolver = resolver(
                config::ResolverConfig::default(),
                config::ResolverOpts::default(),
              ).await;

            let lookup_result = resolver.lookup_ip(address.ip().to_string().as_str()).await?;
            
            if let Some(first_resolved_ip) = lookup_result.iter().next() {
                return Ok(SocketAddr::new(first_resolved_ip, address.port()));
            } else {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "No IPs resolved from the given hostname",
                ));
            }
        }
        Ok(address)
    }
}
