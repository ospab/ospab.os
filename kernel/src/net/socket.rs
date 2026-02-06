//! Socket Interface
//!
//! Provides BSD socket API compatibility.

use super::{IpAddress, Result, NetworkError};
use alloc::vec::Vec;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SocketType {
    Stream,  // TCP
    Dgram,   // UDP
    Raw,     // Raw IP
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SocketDomain {
    AfInet,  // IPv4
}

pub struct Socket {
    pub domain: SocketDomain,
    pub socktype: SocketType,
    pub protocol: i32,
    pub bound_addr: Option<(IpAddress, u16)>,
    pub connected_addr: Option<(IpAddress, u16)>,
}

impl Socket {
    pub fn new(domain: SocketDomain, socktype: SocketType, protocol: i32) -> Result<Self> {
        Ok(Self {
            domain,
            socktype,
            protocol,
            bound_addr: None,
            connected_addr: None,
        })
    }

    pub fn bind(&mut self, addr: IpAddress, port: u16) -> Result<()> {
        self.bound_addr = Some((addr, port));
        crate::serial_print(b"[SOCKET] Socket bound (stub)\r\n");
        Ok(())
    }

    pub fn connect(&mut self, addr: IpAddress, port: u16) -> Result<()> {
        self.connected_addr = Some((addr, port));

        match self.socktype {
            SocketType::Stream => {
                // TCP connect
                let local_addr = self.bound_addr.map(|(a, _)| a).unwrap_or(IpAddress::new(127, 0, 0, 1));
                let local_port = self.bound_addr.map(|(_, p)| p).unwrap_or(0);
                super::tcp::connect(local_addr, local_port, addr, port)?;
            }
            SocketType::Dgram => {
                // UDP is connectionless, just store address
            }
            _ => return Err(NetworkError::NotImplemented),
        }

        Ok(())
    }

    pub fn send(&self, data: &[u8]) -> Result<usize> {
        if let Some((addr, port)) = self.connected_addr {
            match self.socktype {
                SocketType::Stream => {
                    let local_addr = self.bound_addr.map(|(a, _)| a).unwrap_or(IpAddress::new(127, 0, 0, 1));
                    let local_port = self.bound_addr.map(|(_, p)| p).unwrap_or(0);
                    super::tcp::send((local_addr, local_port, addr, port), data)
                }
                SocketType::Dgram => {
                    let src_addr = self.bound_addr.map(|(a, _)| a).unwrap_or(IpAddress::new(127, 0, 0, 1));
                    let src_port = self.bound_addr.map(|(_, p)| p).unwrap_or(0);
                    super::udp::send_to(src_addr, src_port, addr, port, data)?;
                    Ok(data.len())
                }
                _ => Err(NetworkError::NotImplemented),
            }
        } else {
            Err(NetworkError::ConnectionFailed)
        }
    }

    pub fn receive(&self, buffer: &mut [u8]) -> Result<usize> {
        match self.socktype {
            SocketType::Stream => {
                if let Some((addr, port)) = self.connected_addr {
                    let local_addr = self.bound_addr.map(|(a, _)| a).unwrap_or(IpAddress::new(127, 0, 0, 1));
                    let local_port = self.bound_addr.map(|(_, p)| p).unwrap_or(0);
                    super::tcp::receive((local_addr, local_port, addr, port), buffer)
                } else {
                    Err(NetworkError::ConnectionFailed)
                }
            }
            SocketType::Dgram => {
                match super::udp::receive_from(buffer) {
                    Ok((_, _, len)) => Ok(len),
                    Err(e) => Err(e),
                }
            }
            _ => Err(NetworkError::NotImplemented),
        }
    }

    pub fn close(self) -> Result<()> {
        if let (Some((local_addr, local_port)), Some((remote_addr, remote_port))) = (self.bound_addr, self.connected_addr) {
            if self.socktype == SocketType::Stream {
                super::tcp::close((local_addr, local_port, remote_addr, remote_port))?;
            }
        }
        crate::serial_print(b"[SOCKET] Socket closed (stub)\r\n");
        Ok(())
    }
}

// Global socket management
use spin::Mutex;
use alloc::collections::BTreeMap;

static SOCKETS: Mutex<BTreeMap<i32, Socket>> = Mutex::new(BTreeMap::new());
static NEXT_SOCKET_FD: Mutex<i32> = Mutex::new(1);

pub fn socket(domain: SocketDomain, socktype: SocketType, protocol: i32) -> Result<i32> {
    let socket = Socket::new(domain, socktype, protocol)?;
    let fd = {
        let mut next_fd = NEXT_SOCKET_FD.lock();
        let fd = *next_fd;
        *next_fd += 1;
        fd
    };

    SOCKETS.lock().insert(fd, socket);
    Ok(fd)
}

pub fn bind(fd: i32, addr: IpAddress, port: u16) -> Result<()> {
    if let Some(socket) = SOCKETS.lock().get_mut(&fd) {
        socket.bind(addr, port)
    } else {
        Err(NetworkError::InvalidAddress)
    }
}

pub fn connect(fd: i32, addr: IpAddress, port: u16) -> Result<()> {
    if let Some(socket) = SOCKETS.lock().get_mut(&fd) {
        socket.connect(addr, port)
    } else {
        Err(NetworkError::InvalidAddress)
    }
}

pub fn send(fd: i32, data: &[u8]) -> Result<usize> {
    if let Some(socket) = SOCKETS.lock().get(&fd) {
        socket.send(data)
    } else {
        Err(NetworkError::InvalidAddress)
    }
}

pub fn receive(fd: i32, buffer: &mut [u8]) -> Result<usize> {
    if let Some(socket) = SOCKETS.lock().get(&fd) {
        socket.receive(buffer)
    } else {
        Err(NetworkError::InvalidAddress)
    }
}

pub fn close_socket(fd: i32) -> Result<()> {
    if let Some(socket) = SOCKETS.lock().remove(&fd) {
        socket.close()
    } else {
        Err(NetworkError::InvalidAddress)
    }
}