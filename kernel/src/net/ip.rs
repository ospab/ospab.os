//! IP Protocol Implementation
//!
//! Handles IPv4 packet processing and routing.

use super::{IpAddress, Result, NetworkError};
use alloc::vec::Vec;

#[derive(Debug)]
pub struct IpPacket {
    pub version: u8,
    pub ihl: u8,
    pub tos: u8,
    pub total_length: u16,
    pub id: u16,
    pub flags: u8,
    pub fragment_offset: u16,
    pub ttl: u8,
    pub protocol: u8,
    pub checksum: u16,
    pub src_ip: IpAddress,
    pub dst_ip: IpAddress,
    pub payload: Vec<u8>,
}

impl IpPacket {
    pub fn new(src: IpAddress, dst: IpAddress, protocol: u8, payload: Vec<u8>) -> Self {
        let total_length = 20 + payload.len() as u16; // IPv4 header + payload

        Self {
            version: 4,
            ihl: 5,
            tos: 0,
            total_length,
            id: 0,
            flags: 0,
            fragment_offset: 0,
            ttl: 64,
            protocol,
            checksum: 0, // Will be calculated
            src_ip: src,
            dst_ip: dst,
            payload,
        }
    }

    pub fn calculate_checksum(&mut self) {
        // Simple checksum calculation (stub)
        self.checksum = 0x1234;
    }
}

pub struct IpLayer;

impl IpLayer {
    pub fn send_packet(&self, packet: IpPacket) -> Result<()> {
        // Route packet to appropriate interface
        crate::serial_print(b"[IP] Packet sent (stub)\r\n");
        Ok(())
    }

    pub fn receive_packet(&self) -> Option<IpPacket> {
        // Stub - no packets to receive
        None
    }

    pub fn route_packet(&self, dst: IpAddress) -> Result<String> {
        // Simple routing logic
        if dst.0[0] == 127 {
            Ok("lo".to_string())
        } else {
            Ok("eth0".to_string())
        }
    }
}

pub static IP_LAYER: IpLayer = IpLayer;