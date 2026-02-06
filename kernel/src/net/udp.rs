//! UDP Protocol Implementation
//!
//! Provides connectionless, unreliable communication.

use super::{IpAddress, Result, NetworkError};
use alloc::vec::Vec;

#[derive(Debug)]
pub struct UdpPacket {
    pub src_port: u16,
    pub dst_port: u16,
    pub length: u16,
    pub checksum: u16,
    pub payload: Vec<u8>,
}

impl UdpPacket {
    pub fn new(src_port: u16, dst_port: u16, payload: Vec<u8>) -> Self {
        let length = 8 + payload.len() as u16; // UDP header + payload

        Self {
            src_port,
            dst_port,
            length,
            checksum: 0, // Will be calculated
            payload,
        }
    }
}

pub struct UdpSocket;

impl UdpSocket {
    pub fn send_to(&self, src_addr: IpAddress, src_port: u16,
                   dst_addr: IpAddress, dst_port: u16, data: &[u8]) -> Result<()> {
        let packet = UdpPacket::new(src_port, dst_port, data.to_vec());

        // Send via IP layer (stub)
        crate::serial_print(b"[UDP] Packet sent (stub)\r\n");
        Ok(())
    }

    pub fn receive_from(&self, _buffer: &mut [u8]) -> Result<(IpAddress, u16, usize)> {
        // Stub - no data to receive
        Err(NetworkError::Timeout)
    }
}

pub static UDP_SOCKET: UdpSocket = UdpSocket;

pub fn send_to(src_addr: IpAddress, src_port: u16, dst_addr: IpAddress, dst_port: u16, data: &[u8]) -> Result<()> {
    UDP_SOCKET.send_to(src_addr, src_port, dst_addr, dst_port, data)
}

pub fn receive_from(buffer: &mut [u8]) -> Result<(IpAddress, u16, usize)> {
    UDP_SOCKET.receive_from(buffer)
}