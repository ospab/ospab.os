//! TCP Protocol Implementation
//!
//! Provides reliable, connection-oriented communication.

use super::{IpAddress, Result, NetworkError};
use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use spin::Mutex;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TcpState {
    Closed,
    Listen,
    SynSent,
    SynReceived,
    Established,
    FinWait1,
    FinWait2,
    CloseWait,
    Closing,
    LastAck,
    TimeWait,
}

#[derive(Debug)]
pub struct TcpConnection {
    pub local_addr: IpAddress,
    pub local_port: u16,
    pub remote_addr: IpAddress,
    pub remote_port: u16,
    pub state: TcpState,
    pub send_seq: u32,
    pub recv_seq: u32,
}

pub struct TcpSocket {
    connections: BTreeMap<(IpAddress, u16, IpAddress, u16), TcpConnection>,
}

impl TcpSocket {
    pub const fn new() -> Self {
        Self {
            connections: BTreeMap::new(),
        }
    }

    pub fn connect(&mut self, local_addr: IpAddress, local_port: u16,
                   remote_addr: IpAddress, remote_port: u16) -> Result<()> {
        let conn = TcpConnection {
            local_addr,
            local_port,
            remote_addr,
            remote_port,
            state: TcpState::SynSent,
            send_seq: 1000,
            recv_seq: 0,
        };

        self.connections.insert((local_addr, local_port, remote_addr, remote_port), conn);

        // Send SYN packet (stub)
        crate::serial_print(b"[TCP] SYN sent (stub)\r\n");

        // Simulate connection establishment
        if let Some(conn) = self.connections.get_mut(&(local_addr, local_port, remote_addr, remote_port)) {
            conn.state = TcpState::Established;
        }

        Ok(())
    }

    pub fn send(&mut self, _addr: (IpAddress, u16, IpAddress, u16), _data: &[u8]) -> Result<usize> {
        // Stub implementation
        crate::serial_print(b"[TCP] Data sent (stub)\r\n");
        Ok(_data.len())
    }

    pub fn receive(&mut self, _addr: (IpAddress, u16, IpAddress, u16), _buffer: &mut [u8]) -> Result<usize> {
        // Stub - no data to receive
        Err(NetworkError::Timeout)
    }

    pub fn close(&mut self, addr: (IpAddress, u16, IpAddress, u16)) -> Result<()> {
        if let Some(conn) = self.connections.get_mut(&addr) {
            conn.state = TcpState::FinWait1;
            // Send FIN (stub)
            crate::serial_print(b"[TCP] FIN sent (stub)\r\n");
            conn.state = TcpState::Closed;
        }
        Ok(())
    }
}

static TCP_SOCKET: Mutex<TcpSocket> = Mutex::new(TcpSocket::new());

pub fn connect(local_addr: IpAddress, local_port: u16, remote_addr: IpAddress, remote_port: u16) -> Result<()> {
    TCP_SOCKET.lock().connect(local_addr, local_port, remote_addr, remote_port)
}

pub fn send(addr: (IpAddress, u16, IpAddress, u16), data: &[u8]) -> Result<usize> {
    TCP_SOCKET.lock().send(addr, data)
}

pub fn receive(addr: (IpAddress, u16, IpAddress, u16), buffer: &mut [u8]) -> Result<usize> {
    TCP_SOCKET.lock().receive(addr, buffer)
}

pub fn close(addr: (IpAddress, u16, IpAddress, u16)) -> Result<()> {
    TCP_SOCKET.lock().close(addr)
}