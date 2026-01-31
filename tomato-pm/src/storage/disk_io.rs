use std::fs;
use std::io;
use std::path::Path;

pub struct PackageDB {
    path: String,
}

impl PackageDB {
    pub fn new(path: &str) -> Self {
        PackageDB {
            path: path.to_string(),
        }
    }

    pub fn load_installed(&self) -> io::Result<Vec<String>> {
        if Path::new(&self.path).exists() {
            let content = fs::read_to_string(&self.path)?;
            Ok(content.lines().map(|s| s.to_string()).collect())
        } else {
            Ok(Vec::new())
        }
    }

    pub fn save_installed(&self, packages: &[String]) -> io::Result<()> {
        let content = packages.join("\n");
        fs::write(&self.path, content)
    }

    pub fn is_installed(&self, package: &str) -> io::Result<bool> {
        let installed = self.load_installed()?;
        Ok(installed.contains(&package.to_string()))
    }
}