use std::collections::{HashMap, HashSet};

pub fn resolve_dependencies(package: &str, available: &HashMap<String, Vec<String>>) -> Result<Vec<String>, String> {
    let mut resolved = Vec::new();
    let mut seen = HashSet::new();
    let mut to_resolve = vec![package.to_string()];

    while let Some(pkg) = to_resolve.pop() {
        if seen.contains(&pkg) {
            continue;
        }
        seen.insert(pkg.clone());

        if let Some(deps) = available.get(&pkg) {
            for dep in deps {
                if !seen.contains(dep) {
                    to_resolve.push(dep.clone());
                }
            }
        }

        resolved.push(pkg);
    }

    // Reverse to get installation order
    resolved.reverse();
    Ok(resolved)
}