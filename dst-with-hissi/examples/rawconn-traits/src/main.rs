//! RawConn Trait Example
//!
//! Demonstrates how to write code that works with both
//! real I/O (tokio) and simulated I/O.

use std::net::SocketAddr;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

/// Generic service that works with any RawConn implementation
pub struct EchoService<C: foundation_core::io::RawConn> {
    _phantom: std::marker::PhantomData<C>,
}

impl<C: foundation_core::io::RawConn> EchoService<C> {
    pub fn new() -> Self {
        Self {
            _phantom: std::marker::PhantomData,
        }
    }

    /// Run an echo server
    pub async fn run_server(&self, addr: SocketAddr) -> std::io::Result<()> {
        println!("Starting echo server on {}", addr);
        
        let listener = C::bind_listen(addr).await?;
        futures::pin_mut!(listener);
        
        // Accept connections
        while let Some(result) = listener.next().await {
            let stream = result?;
            println!("New connection");
            
            // Handle each connection in a separate task
            tokio::spawn(async move {
                if let Err(e) = handle_echo(stream).await {
                    eprintln!("Connection error: {}", e);
                }
            });
        }
        
        Ok(())
    }

    /// Run an echo client
    pub async fn run_client(&self, addr: SocketAddr, message: &str) -> std::io::Result<String> {
        println!("Connecting to {}", addr);
        
        let mut stream = C::connect(addr).await?;
        
        // Send message
        stream.write_all(message.as_bytes()).await?;
        
        // Read response
        let mut response = vec![0u8; 1024];
        let n = stream.read(&mut response).await?;
        
        Ok(String::from_utf8_lossy(&response[..n]).to_string())
    }
}

async fn handle_echo<S: AsyncReadExt + AsyncWriteExt + Unpin>(mut stream: S) -> std::io::Result<()> {
    let mut buf = vec![0u8; 1024];
    let n = stream.read(&mut buf).await?;
    
    if n > 0 {
        println!("Received: {}", String::from_utf8_lossy(&buf[..n]));
        stream.write_all(&buf[..n]).await?;
    }
    
    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    println!("RawConn Example");
    println!("===============");
    println!();
    
    let service = EchoService::<foundation_core::io::RawConnImpl>::new();
    let addr: SocketAddr = "127.0.0.1:8765".parse()?;
    
    // In production mode, this uses tokio
    // In simulation mode (cargo test --features simulation), uses SimConn
    
    println!("Running echo server test...");
    
    // Spawn server
    let server_handle = tokio::spawn(async move {
        service.run_server(addr).await
    });
    
    // Give server time to start
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    
    // Run client
    let response = service.run_client(addr, "Hello, World!").await?;
    println!("Client received: {}", response);
    
    // Cleanup
    server_handle.abort();
    
    println!();
    println!("Test complete!");
    println!();
    println!("To run with simulation:");
    println!("  cargo test --features simulation");
    
    Ok(())
}
