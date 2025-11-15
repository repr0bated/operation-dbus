use crate::types::Package;
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

/// Parse a Debian Packages file
pub fn parse_packages_file<P: AsRef<Path>>(path: P) -> Result<HashMap<String, Package>> {
    let file = File::open(path.as_ref())
        .context(format!("Failed to open {}", path.as_ref().display()))?;
    let reader = BufReader::new(file);

    let mut packages = HashMap::new();
    let mut current_pkg: Option<Package> = None;
    let mut current_field = String::new();

    for line in reader.lines() {
        let line = line?;

        // Empty line = end of package stanza
        if line.trim().is_empty() {
            if let Some(pkg) = current_pkg.take() {
                packages.insert(pkg.name.clone(), pkg);
            }
            current_field.clear();
            continue;
        }

        // Continuation line (starts with space)
        if line.starts_with(' ') {
            if current_field == "Description" {
                if let Some(ref mut pkg) = current_pkg {
                    pkg.description.push('\n');
                    pkg.description.push_str(line.trim());
                }
            }
            continue;
        }

        // New field
        if let Some((field, value)) = line.split_once(':') {
            let field = field.trim();
            let value = value.trim();

            if current_pkg.is_none() {
                current_pkg = Some(Package::default());
            }

            current_field = field.to_string();

            if let Some(ref mut pkg) = current_pkg {
                match field {
                    "Package" => pkg.name = value.to_string(),
                    "Version" => pkg.version = value.to_string(),
                    "Architecture" => pkg.architecture = value.to_string(),
                    "Depends" => pkg.depends = parse_dependency_list(value),
                    "Pre-Depends" => pkg.pre_depends = parse_dependency_list(value),
                    "Recommends" => pkg.recommends = parse_dependency_list(value),
                    "Suggests" => pkg.suggests = parse_dependency_list(value),
                    "Conflicts" => pkg.conflicts = parse_dependency_list(value),
                    "Replaces" => pkg.replaces = parse_dependency_list(value),
                    "Provides" => pkg.provides = parse_dependency_list(value),
                    "Essential" => pkg.essential = value.eq_ignore_ascii_case("yes"),
                    "Priority" => pkg.priority = value.to_string(),
                    "Section" => pkg.section = value.to_string(),
                    "Description" => pkg.description = value.to_string(),
                    "Filename" => pkg.filename = value.to_string(),
                    "Size" => pkg.size = value.parse().unwrap_or(0),
                    "MD5sum" => pkg.md5sum = value.to_string(),
                    "SHA256" => pkg.sha256 = value.to_string(),
                    _ => {}
                }
            }
        }
    }

    // Don't forget last package
    if let Some(pkg) = current_pkg {
        packages.insert(pkg.name.clone(), pkg);
    }

    Ok(packages)
}

/// Parse dependency list string
fn parse_dependency_list(deps: &str) -> Vec<String> {
    deps.split(',')
        .map(|dep| {
            dep.trim()
                // Remove version constraints
                .split_once('(')
                .map(|(name, _)| name.trim())
                .unwrap_or(dep.trim())
                // Remove architecture constraints
                .split_once('[')
                .map(|(name, _)| name.trim())
                .unwrap_or_else(|| {
                    dep.trim()
                        .split_once('(')
                        .map(|(name, _)| name.trim())
                        .unwrap_or(dep.trim())
                })
                // Take first alternative
                .split('|')
                .next()
                .unwrap_or("")
                .trim()
                .to_string()
        })
        .filter(|s| !s.is_empty())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_dependency_list() {
        let deps = "libc6 (>= 2.34), bash (>= 5.0) | dash, libssl3:amd64";
        let parsed = parse_dependency_list(deps);
        assert_eq!(parsed, vec!["libc6", "bash", "libssl3"]);
    }
}
