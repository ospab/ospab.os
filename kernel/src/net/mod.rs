//! Network Stack for ospabOS
//!
//! Provides TCP/IP networking with socket interface.
//! Currently implements basic stub networking for demonstration.

pub mod ethernet;
pub mod ip;
pub mod tcp;
pub mod udp;
pub mod socket;
pub mod dns;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use spin::Mutex;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NetworkError {
    NoDevice,
    InvalidAddress,
    ConnectionFailed,
    Timeout,
    BufferTooSmall,
    NotImplemented,
}

pub type Result<T> = core::result::Result<T, NetworkError>;

#[derive(Debug, Clone, Copy)]
pub struct MacAddress([u8; 6]);

impl MacAddress {
    pub const fn new(bytes: [u8; 6]) -> Self {
        Self(bytes)
    }

    pub fn bytes(&self) -> &[u8; 6] {
        &self.0
    }
}

#[derive(Debug, Clone, Copy)]
pub struct IpAddress([u8; 4]);

impl IpAddress {
    pub const fn new(a: u8, b: u8, c: u8, d: u8) -> Self {
        Self([a, b, c, d])
    }

    pub fn bytes(&self) -> &[u8; 4] {
        &self.0
    }

    pub fn from_bytes(bytes: [u8; 4]) -> Self {
        Self(bytes)
    }
}

#[derive(Debug, Clone)]
pub struct NetworkInterface {
    pub name: String,
    pub mac: MacAddress,
    pub ip: IpAddress,
    pub netmask: IpAddress,
    pub gateway: IpAddress,
    pub mtu: u16,
}

pub struct NetworkStack {
    interfaces: BTreeMap<String, NetworkInterface>,
}

impl NetworkStack {
    pub const fn new() -> Self {
        Self {
            interfaces: BTreeMap::new(),
        }
    }

    pub fn add_interface(&mut self, iface: NetworkInterface) {
        self.interfaces.insert(iface.name.clone(), iface);
    }

    pub fn get_interface(&self, name: &str) -> Option<&NetworkInterface> {
        self.interfaces.get(name)
    }

    pub fn list_interfaces(&self) -> Vec<&NetworkInterface> {
        self.interfaces.values().collect()
    }
}

static NETWORK_STACK: Mutex<NetworkStack> = Mutex::new(NetworkStack::new());

pub fn init() {
    let mut stack = NETWORK_STACK.lock();

    // Create a loopback interface
    let lo = NetworkInterface {
        name: "lo".to_string(),
        mac: MacAddress::new([0, 0, 0, 0, 0, 0]),
        ip: IpAddress::new(127, 0, 0, 1),
        netmask: IpAddress::new(255, 0, 0, 0),
        gateway: IpAddress::new(0, 0, 0, 0),
        mtu: 65536,
    };
    stack.add_interface(lo);

    // Create a dummy ethernet interface
    let eth0 = NetworkInterface {
        name: "eth0".to_string(),
        mac: MacAddress::new([0x52, 0x54, 0x00, 0x12, 0x34, 0x56]),
        ip: IpAddress::new(192, 168, 1, 100),
        netmask: IpAddress::new(255, 255, 255, 0),
        gateway: IpAddress::new(192, 168, 1, 1),
        mtu: 1500,
    };
    stack.add_interface(eth0);

    crate::serial_print(b"[NET] Network stack initialized\r\n");
    crate::serial_print(b"[NET] Interfaces: lo (127.0.0.1), eth0 (192.168.1.100)\r\n");
}

pub fn get_interface(name: &str) -> Option<NetworkInterface> {
    NETWORK_STACK.lock().get_interface(name).cloned()
}

pub fn list_interfaces() -> Vec<NetworkInterface> {
    NETWORK_STACK.lock().list_interfaces().into_iter().cloned().collect()
}

// Stub implementations for networking functions
pub fn ping(address: IpAddress, timeout_ms: u32) -> Result<u32> {
    // Simulate ping - always succeed for demo
    crate::drivers::timer::sleep_ms(timeout_ms / 2);
    Ok(timeout_ms / 2)
}

pub fn resolve_hostname(hostname: &str) -> Result<IpAddress> {
    // Simple stub DNS resolution
    match hostname {
        "localhost" => Ok(IpAddress::new(127, 0, 0, 1)),
        "google.com" => Ok(IpAddress::new(8, 8, 8, 8)),
        _ => Err(NetworkError::InvalidAddress),
    }
}