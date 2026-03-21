//! I/O Dispatcher Example (Hiisi Pattern)
//!
//! Demonstrates the callback-based I/O dispatcher pattern
//! used by Hiisi and TigerBeetle for deterministic simulation.

use bytes::Bytes;
use polling::{Event, Events, Poller};
use socket2::{Domain, Socket, Type, SockAddr};
use std::{
    cell::RefCell,
    collections::{HashMap, VecDeque},
    io,
    os::fd::AsRawFd,
    rc::Rc,
};
use tracing::{info, trace, debug};

// ============ Completion Types ============

type AcceptCallback = fn(&mut IoDispatcher, Rc<Socket>, SockAddr, Rc<Socket>, SockAddr);
type RecvCallback = fn(&mut IoDispatcher, Rc<Socket>, &[u8], usize);
type SendCallback = fn(&mut IoDispatcher, Rc<Socket>, usize);

enum Completion {
    Accept {
        server_sock: Rc<Socket>,
        server_addr: SockAddr,
        cb: AcceptCallback,
    },
    Recv {
        sock: Rc<Socket>,
        cb: RecvCallback,
    },
    Send {
        sock: Rc<Socket>,
        buf: Bytes,
        n: usize,
        cb: SendCallback,
    },
}

impl std::fmt::Debug for Completion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Completion::Accept { .. } => write!(f, "Accept"),
            Completion::Recv { .. } => write!(f, "Recv"),
            Completion::Send { .. } => write!(f, "Send"),
        }
    }
}

// ============ I/O Dispatcher ============

/// I/O Dispatcher - heart of the architecture
///
/// Instead of blocking on I/O, operations register callbacks
/// and the dispatcher processes completions in run_once().
pub struct IoDispatcher {
    poller: Poller,
    events: Events,
    key_seq: usize,
    submissions: HashMap<usize, Completion>,
    completions: VecDeque<Completion>,
}

impl IoDispatcher {
    pub fn new() -> io::Result<Self> {
        Ok(Self {
            poller: Poller::new()?,
            events: Events::new(),
            key_seq: 0,
            submissions: HashMap::new(),
            completions: VecDeque::new(),
        })
    }

    /// Run one iteration of the I/O loop
    pub fn run_once(&mut self) {
        self.events.clear();
        
        // Wait for events with timeout
        let _ = self.poller.wait(
            &mut self.events,
            Some(std::time::Duration::from_micros(500)),
        );

        self.flush_submissions();
        self.flush_completions();
    }

    fn flush_submissions(&mut self) {
        debug!("Flushing {} submissions", self.events.len());
        
        for event in self.events.iter() {
            trace!("Event key: {}", event.key);
            
            if let Some(c) = self.submissions.remove(&event.key) {
                c.prepare();
                
                // Remove from poller
                match &c {
                    Completion::Accept { server_sock, .. } => {
                        let _ = self.poller.delete(server_sock);
                    }
                    Completion::Recv { sock, .. } => {
                        let _ = self.poller.delete(sock);
                    }
                    Completion::Send { sock, .. } => {
                        let _ = self.poller.delete(sock);
                    }
                }
                
                self.completions.push_back(c);
            }
        }
    }

    fn flush_completions(&mut self) {
        debug!("Flushing {} completions", self.completions.len());
        
        loop {
            if let Some(c) = self.completions.pop_front() {
                c.complete(self);
            } else {
                break;
            }
        }
    }

    fn get_key(&mut self) -> usize {
        let key = self.key_seq;
        self.key_seq += 1;
        key
    }

    fn enqueue(&mut self, key: usize, c: Completion) {
        self.submissions.insert(key, c);
    }

    /// Accept a connection (non-blocking)
    pub fn accept(&mut self, server_sock: Rc<Socket>, server_addr: SockAddr, cb: AcceptCallback) {
        let sockfd = server_sock.as_raw_fd();
        trace!("Accept on sockfd {}", sockfd);

        let c = Completion::Accept {
            server_sock,
            server_addr,
            cb,
        };

        let key = self.get_key();
        
        // Register with poller for readability
        unsafe {
            let _ = self.poller.add(&c, Event::readable(key));
        }
        
        self.enqueue(key, c);
    }

    /// Receive data (non-blocking)
    pub fn recv(&mut self, sock: Rc<Socket>, cb: RecvCallback) {
        let sockfd = sock.as_raw_fd();
        trace!("Recv on sockfd {}", sockfd);

        let c = Completion::Recv { sock, cb };
        let key = self.get_key();

        unsafe {
            let _ = self.poller.add(&c, Event::readable(key));
        }

        self.enqueue(key, c);
    }

    /// Send data (non-blocking)
    pub fn send(&mut self, sock: Rc<Socket>, buf: Bytes, n: usize, cb: SendCallback) {
        let sockfd = sock.as_raw_fd();
        trace!("Send on sockfd {}", sockfd);

        let c = Completion::Send { sock, buf, n, cb };
        let key = self.get_key();

        unsafe {
            let _ = self.poller.add(&c, Event::writable(key));
        }

        self.enqueue(key, c);
    }

    /// Close a socket
    pub fn close(&mut self, sock: Rc<Socket>) {
        let sockfd = sock.as_raw_fd();
        trace!("Close sockfd {}", sockfd);
        drop(sock);
    }
}

impl Completion {
    fn prepare(&self) {
        // Prepare for execution (if needed)
    }

    fn complete(self, io: &mut IoDispatcher) {
        match self {
            Completion::Accept {
                server_sock,
                server_addr,
                cb,
            } => {
                // Actually accept the connection
                match server_sock.accept() {
                    Ok((sock, sock_addr)) => {
                        cb(io, server_sock, server_addr, Rc::new(sock), sock_addr);
                    }
                    Err(e) => {
                        if e.kind() != io::ErrorKind::WouldBlock {
                            tracing::error!("Accept error: {}", e);
                        }
                    }
                }
            }
            Completion::Recv { sock, cb } => {
                let mut buf = vec![0u8; 4096];
                match sock.recv(&mut buf) {
                    Ok(n) => {
                        if n > 0 {
                            cb(io, sock, &buf[..n], n);
                        } else {
                            trace!("Connection closed");
                            io.close(sock);
                        }
                    }
                    Err(e) => {
                        if e.kind() != io::ErrorKind::WouldBlock {
                            tracing::error!("Recv error: {}", e);
                        }
                    }
                }
            }
            Completion::Send { sock, buf, n, cb } => {
                match sock.send(&buf[..n]) {
                    Ok(sent_n) => {
                        cb(io, sock, sent_n);
                    }
                    Err(e) => {
                        if e.kind() != io::ErrorKind::WouldBlock {
                            tracing::error!("Send error: {}", e);
                        }
                    }
                }
            }
        }
    }
}

// ============ Echo Server Example ============

/// Echo server using I/O dispatcher
pub struct EchoServer {
    dispatcher: IoDispatcher,
}

impl EchoServer {
    pub fn new() -> io::Result<Self> {
        Ok(Self {
            dispatcher: IoDispatcher::new()?,
        })
    }

    pub fn run(&mut self, addr: SocketAddr) -> io::Result<()> {
        // Create listener socket
        let sock = Rc::new(Socket::new(Domain::IPV4, Type::STREAM, None)?);
        sock.set_reuse_address(true)?;
        sock.bind(&addr.into())?;
        sock.listen(128)?;

        let sock_addr: SockAddr = addr.into();
        
        info!("Echo server listening on {}", addr);

        // Start accepting
        self.dispatcher.accept(sock, sock_addr, on_accept);

        // Run I/O loop
        loop {
            self.dispatcher.run_once();
        }
    }
}

fn on_accept(
    io: &mut IoDispatcher,
    server_sock: Rc<Socket>,
    server_addr: SockAddr,
    conn_sock: Rc<Socket>,
    client_addr: SockAddr,
) {
    trace!("Accepted connection from {:?}", client_addr);
    
    // Re-arm accept for next connection
    io.accept(server_sock, server_addr, on_accept);
    
    // Start receiving on new connection
    io.recv(conn_sock, on_recv);
}

fn on_recv(io: &mut IoDispatcher, sock: Rc<Socket>, buf: &[u8], n: usize) {
    trace!("Received {} bytes", n);
    
    if n == 0 {
        trace!("Client closed connection");
        io.close(sock);
        return;
    }

    // Echo back
    let response = Bytes::copy_from_slice(buf);
    io.send(sock, response, n, on_send);
}

fn on_send(io: &mut IoDispatcher, sock: Rc<Socket>, _n: usize) {
    // Re-arm recv for next message
    io.recv(sock, on_recv);
}

// ============ Main ============

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("io_dispatcher=info".parse().unwrap()),
        )
        .init();

    info!("I/O Dispatcher Example (Hiisi Pattern)");
    info!("======================================");

    let addr: SocketAddr = "127.0.0.1:9876".parse()?;
    
    info!("Starting echo server on {}", addr);
    info!("Test with: echo 'Hello' | nc localhost 9876");
    info!("Press Ctrl+C to stop");

    let mut server = EchoServer::new()?;
    server.run(addr)?;

    Ok(())
}
