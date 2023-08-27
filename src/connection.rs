use crate::errors::{SessionError, SessionResult};
use crate::Config;
use async_std::{
    io::prelude::*,
    net::{SocketAddr, TcpListener, TcpStream},
};
use async_std_resolver::config::{ResolverConfig, ResolverOpts};
use trust_dns_resolver::Resolver; // TODO: Figure out async DNS
use std::sync::Arc;
use log::{error, info};

pub struct Connection {
    stream: TcpStream,
    config: Arc<Config>,
}

impl Connection {
    pub async fn from_config(config: Arc<Config>, address: SocketAddr) -> SessionResult<Self> {
        // TODO: Implement "dont_resolve" flag (skips DNS)
        let address = Self::resolve_hostname(address)?;

        if config.listen {
            Self::listen(config, address).await
        } else {
            info!("Connecting to {}", address);
            let stream = TcpStream::connect(&address)
                .await
                .map_err(SessionError::from)?;
            info!("Connected to {}", address);
            stream.set_nodelay(true).map_err(SessionError::from)?;

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
                return Err(SessionError::ClientDisconnected);
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

    fn resolve_hostname(address: SocketAddr) -> Result<SocketAddr, std::io::Error> {
        if address.ip().is_unspecified() {
            let resolver = Resolver::new(ResolverConfig::default(), ResolverOpts::default())?;
            let lookup_result = resolver.lookup_ip(address.ip().to_string().as_str())?;
            
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
