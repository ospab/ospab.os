//! Ethernet Driver Stub
//!
//! Provides basic ethernet frame handling.
//! Currently just a stub for demonstration.

use super::{MacAddress, Result, NetworkError};

pub struct EthernetFrame {
    pub dst_mac: MacAddress,
    pub src_mac: MacAddress,
    pub ethertype: u16,
    pub payload: Vec<u8>,
}

impl EthernetFrame {
    pub fn new(dst: MacAddress, src: MacAddress, ethertype: u16, payload: Vec<u8>) -> Self {
        Self {
            dst_mac: dst,
            src_mac: src,
            ethertype,
            payload,
        }
    }
}

pub struct EthernetDriver;

impl EthernetDriver {
    pub fn send_frame(&self, _frame: EthernetFrame) -> Result<()> {
        // Stub implementation - just log
        crate::serial_print(b"[ETH] Frame sent (stub)\r\n");
        Ok(())
    }

    pub fn receive_frame(&self) -> Option<EthernetFrame> {
        // Stub - no frames to receive
        None
    }
}

pub static ETHERNET_DRIVER: EthernetDriver = EthernetDriver;