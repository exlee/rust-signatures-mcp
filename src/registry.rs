use std::fs;
use std::path::PathBuf;

pub fn find_package_dir(package: &str, version: Option<&str>) -> Result<PathBuf, String> {
    let registry_src = cargo_registry_src();
    let normalized = package.replace('_', "-");

    let index_dirs: Vec<PathBuf> = match fs::read_dir(&registry_src) {
        Ok(entries) => entries
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.file_name()
                    .to_str()
                    .map(|n| n.starts_with("index.crates.io-"))
                    .unwrap_or(false)
            })
            .map(|e| e.path())
            .collect(),
        Err(e) => return Err(format!("Cannot read cargo registry src: {}", e)),
    };

    if index_dirs.is_empty() {
        return Err("No crates.io index found in cargo registry. Run cargo build first.".into());
    }

    let prefix = format!("{}-", normalized);

    for index_dir in &index_dirs {
        let entries: Vec<_> = match fs::read_dir(index_dir) {
            Ok(entries) => entries.filter_map(|e| e.ok()).collect(),
            Err(_) => continue,
        };

        let matching: Vec<(PathBuf, String)> = entries
            .into_iter()
            .filter_map(|e| {
                let name = e.file_name().to_str()?.to_string();
                let ver = name.strip_prefix(&prefix)?.to_string();
                if !e.path().is_dir() {
                    return None;
                }
                Some((e.path(), ver))
            })
            .collect();

        if matching.is_empty() {
            continue;
        }

        if let Some(ver) = version {
            if let Some((path, _)) = matching.iter().find(|(_, v)| v.starts_with(ver)) {
                return Ok(path.clone());
            }
        } else {
            let best = matching
                .into_iter()
                .filter(|(_, v)| {
                    v.split('+')
                        .next()
                        .and_then(|sv| semver::Version::parse(sv).ok())
                        .is_some()
                })
                .max_by(|a, b| {
                    let va = semver::Version::parse(a.1.split('+').next().unwrap_or(&a.1)).ok();
                    let vb = semver::Version::parse(b.1.split('+').next().unwrap_or(&b.1)).ok();
                    match (va, vb) {
                        (Some(a), Some(b)) => a.cmp(&b),
                        (Some(_), None) => std::cmp::Ordering::Greater,
                        (None, Some(_)) => std::cmp::Ordering::Less,
                        (None, None) => std::cmp::Ordering::Equal,
                    }
                });

            if let Some((path, _ver)) = best {
                return Ok(path);
            }
        }
    }

    match version {
        Some(v) => Err(format!(
            "Package {} v{} not found in cargo cache",
            package, v
        )),
        None => Err(format!("Package {} not found in cargo cache", package)),
    }
}

fn cargo_registry_src() -> PathBuf {
    let cargo_home = std::env::var("CARGO_HOME").unwrap_or_else(|_| {
        std::env::var("HOME")
            .map(|h| format!("{}/.cargo", h))
            .unwrap_or_else(|_| ".cargo".into())
    });
    PathBuf::from(cargo_home).join("registry/src")
}
