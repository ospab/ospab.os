//! DNS Resolution Stub
//!
//! Provides hostname to IP address resolution.

use super::{IpAddress, Result, NetworkError};
use alloc::collections::BTreeMap;
use spin::Mutex;

static DNS_CACHE: Mutex<BTreeMap<&'static str, IpAddress>> = Mutex::new(BTreeMap::new());

pub fn init() {
    let mut cache = DNS_CACHE.lock();
    cache.insert("localhost", IpAddress::new(127, 0, 0, 1));
    cache.insert("google.com", IpAddress::new(8, 8, 8, 8));
    cache.insert("cloudflare.com", IpAddress::new(1, 1, 1, 1));
}

pub fn resolve(hostname: &str) -> Result<IpAddress> {
    // Check cache first
    if let Some(ip) = DNS_CACHE.lock().get(hostname) {
        return Ok(*ip);
    }

    // Stub DNS resolution - return error for unknown hosts
    Err(NetworkError::InvalidAddress)
}

pub fn add_to_cache(hostname: &'static str, ip: IpAddress) {
    DNS_CACHE.lock().insert(hostname, ip);
}