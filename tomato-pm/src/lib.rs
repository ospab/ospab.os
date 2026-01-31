use std::collections::HashMap;
use crate::api::cli::{parse_command, Command};
use crate::storage::disk_io::PackageDB;
use crate::core::solver::resolve_dependencies;
use crate::parser::toml::parse_toml;

pub mod core;
pub mod storage;
pub mod api;
pub mod parser;

pub fn run(args: &[String]) {
    let db = PackageDB::new("/var/lib/tomato/packages.txt");
    match parse_command(args) {
        Ok(Command::Install(pkg)) => {
            let mut available = HashMap::new();
            // Load available packages from /var/lib/tomato/available.toml
            if let Ok(content) = std::fs::read_to_string("/var/lib/tomato/available.toml") {
                if let Ok(parsed) = parse_toml(&content) {
                    for (key, value) in &parsed {
                        if key.ends_with(".deps") {
                            let pkg_name = key.trim_end_matches(".deps");
                            let deps: Vec<String> = value.split(',').map(|s: &str| s.trim().to_string()).collect();
                            available.insert(pkg_name.to_string(), deps);
                        }
                    }
                }
            }
            // Add defaults
            available.entry("base".to_string()).or_insert(vec![]);
            available.entry("kernel".to_string()).or_insert(vec!["base".to_string()]);

            match resolve_dependencies(&pkg, &available) {
                Ok(deps) => {
                    match db.load_installed() {
                        Ok(mut installed) => {
                            for dep in deps {
                                if !installed.contains(&dep) {
                                    println!("Installing {}", dep);
                                    installed.push(dep);
                                }
                            }
                            if let Err(e) = db.save_installed(&installed) {
                                println!("Error saving: {}", e);
                            }
                        }
                        Err(e) => println!("Error loading: {}", e),
                    }
                }
                Err(e) => println!("Dependency error: {}", e),
            }
        }
        Ok(Command::Remove(pkg)) => {
            match db.load_installed() {
                Ok(mut installed) => {
                    if let Some(pos) = installed.iter().position(|p| p == &pkg) {
                        installed.remove(pos);
                        if let Err(e) = db.save_installed(&installed) {
                            println!("Error saving: {}", e);
                        } else {
                            println!("Removed {}", pkg);
                        }
                    } else {
                        println!("Package {} not installed", pkg);
                    }
                }
                Err(e) => println!("Error loading: {}", e),
            }
        }
        Ok(Command::List) => {
            match db.load_installed() {
                Ok(installed) => {
                    if installed.is_empty() {
                        println!("No packages installed");
                    } else {
                        for pkg in installed {
                            println!("{}", pkg);
                        }
                    }
                }
                Err(e) => println!("Error loading: {}", e),
            }
        }
        Ok(Command::Search(query)) => {
            // Simple search in available
            let mut available: HashMap<String, Vec<String>> = HashMap::new();
            if let Ok(content) = std::fs::read_to_string("/var/lib/tomato/available.toml") {
                if let Ok(parsed) = parse_toml(&content) {
                    for (key, _) in &parsed {
                        if key.contains(&query) {
                            println!("{}", key);
                        }
                    }
                }
            }
        }
        Err(e) => println!("Command error: {}", e),
    }
}