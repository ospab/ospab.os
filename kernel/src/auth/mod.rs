//! User Authentication and Authorization System for ospabOS
//!
//! Provides user management, authentication, and access control.
//! Currently supports a simple user database with basic permissions.

use alloc::collections::BTreeMap;
use alloc::string::{String, ToString};
use alloc::vec;
use alloc::vec::Vec;
use alloc::format;
use core::sync::atomic::{AtomicU32, Ordering};
use spin::Mutex;

pub static CURRENT_USER: AtomicU32 = AtomicU32::new(0); // 0 = root

#[derive(Debug, Clone, PartialEq)]
pub enum Permission {
    Read,
    Write,
    Execute,
    Admin,
}

#[derive(Debug, Clone)]
pub struct User {
    pub id: u32,
    pub name: String,
    pub password_hash: u32, // Simple hash for now
    pub permissions: Vec<Permission>,
    pub home_dir: String,
}

impl User {
    pub fn new(id: u32, name: &str, password: &str) -> Self {
        Self {
            id,
            name: name.to_string(),
            password_hash: simple_hash(password),
            permissions: if id == 0 {
                vec![Permission::Read, Permission::Write, Permission::Execute, Permission::Admin]
            } else {
                vec![Permission::Read, Permission::Write, Permission::Execute]
            },
            home_dir: if id == 0 {
                "/root".to_string()
            } else {
                format!("/home/{}", name)
            },
        }
    }

    pub fn has_permission(&self, perm: &Permission) -> bool {
        self.permissions.contains(perm)
    }

    pub fn check_password(&self, password: &str) -> bool {
        simple_hash(password) == self.password_hash
    }
}

pub struct UserManager {
    users: BTreeMap<u32, User>,
    users_by_name: BTreeMap<String, u32>,
    next_id: u32,
}

impl UserManager {
    pub const fn new() -> Self {
        Self {
            users: BTreeMap::new(),
            users_by_name: BTreeMap::new(),
            next_id: 1, // 0 is reserved for root
        }
    }

    pub fn init(&mut self) {
        // Create root user
        let root = User::new(0, "root", "root");
        self.users.insert(0, root.clone());
        self.users_by_name.insert("root".to_string(), 0);

        // Create default user
        let user = User::new(1, "ospab", "ospab");
        self.users.insert(1, user.clone());
        self.users_by_name.insert("ospab".to_string(), 1);
        self.next_id = 2;
    }

    pub fn authenticate(&self, username: &str, password: &str) -> Option<&User> {
        if let Some(user_id) = self.users_by_name.get(username) {
            if let Some(user) = self.users.get(user_id) {
                if user.check_password(password) {
                    return Some(user);
                }
            }
        }
        None
    }

    pub fn get_user(&self, id: u32) -> Option<&User> {
        self.users.get(&id)
    }

    pub fn get_user_by_name(&self, name: &str) -> Option<&User> {
        self.users_by_name.get(name)
            .and_then(|id| self.users.get(id))
    }

    pub fn current_user(&self) -> Option<&User> {
        let uid = CURRENT_USER.load(Ordering::Relaxed);
        self.get_user(uid)
    }

    pub fn switch_user(&self, username: &str, password: &str) -> Result<(), &'static str> {
        if let Some(user) = self.authenticate(username, password) {
            CURRENT_USER.store(user.id, Ordering::Relaxed);
            Ok(())
        } else {
            Err("Authentication failed")
        }
    }

    pub fn add_user(&mut self, name: &str, password: &str) -> Result<u32, &'static str> {
        if self.users_by_name.contains_key(name) {
            return Err("User already exists");
        }

        let id = self.next_id;
        self.next_id += 1;

        let user = User::new(id, name, password);
        self.users.insert(id, user.clone());
        self.users_by_name.insert(name.to_string(), id);

        Ok(id)
    }

    pub fn list_users(&self) -> Vec<&User> {
        self.users.values().collect()
    }
}

static USER_MANAGER: Mutex<UserManager> = Mutex::new(UserManager::new());

pub fn init() {
    USER_MANAGER.lock().init();
    crate::serial_print!(b"[AUTH] User authentication system initialized\r\n");
}

pub fn authenticate(username: &str, password: &str) -> Option<User> {
    USER_MANAGER.lock().authenticate(username, password).cloned()
}

pub fn current_user() -> Option<User> {
    USER_MANAGER.lock().current_user().cloned()
}

pub fn switch_user(username: &str, password: &str) -> Result<(), &'static str> {
    USER_MANAGER.lock().switch_user(username, password)
}

pub fn add_user(name: &str, password: &str) -> Result<u32, &'static str> {
    USER_MANAGER.lock().add_user(name, password)
}

pub fn list_users() -> Vec<User> {
    USER_MANAGER.lock().list_users().into_iter().cloned().collect()
}

pub fn check_permission(user_id: u32, perm: Permission) -> bool {
    if let Some(user) = USER_MANAGER.lock().get_user(user_id) {
        user.has_permission(&perm)
    } else {
        false
    }
}

pub fn current_user_id() -> u32 {
    CURRENT_USER.load(Ordering::Relaxed)
}

pub fn current_username() -> String {
    if let Some(user) = current_user() {
        user.name
    } else {
        "unknown".to_string()
    }
}

// Simple hash function for passwords (NOT secure, just for demo)
fn simple_hash(s: &str) -> u32 {
    let mut hash: u32 = 0;
    for byte in s.bytes() {
        hash = hash.wrapping_mul(31).wrapping_add(byte as u32);
    }
    hash
}