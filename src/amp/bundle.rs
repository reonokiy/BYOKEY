use std::{path::PathBuf, process::Stdio};

/// Discover patchable Amp bundles, optionally including editor extensions.
pub fn find_amp_bundles(extension: bool) -> Vec<PathBuf> {
    let mut seen = std::collections::HashSet::new();
    let mut result = Vec::new();

    for p in find_cli_binary() {
        if seen.insert(p.clone()) {
            result.push(p);
        }
    }

    if extension {
        for p in find_extension_bundles() {
            if seen.insert(p.clone()) {
                result.push(p);
            }
        }
    }

    result
}

/// Find the Amp CLI binary on PATH.
fn find_cli_binary() -> Vec<PathBuf> {
    let mut result = Vec::new();
    if let Ok(output) = std::process::Command::new("which")
        .arg("amp")
        .stderr(Stdio::null())
        .output()
        && let Ok(raw) = std::str::from_utf8(&output.stdout)
    {
        let raw = raw.trim();
        if !raw.is_empty()
            && let Ok(real) = std::fs::canonicalize(raw)
            && has_ad_code(&real)
        {
            result.push(real);
        }
    }
    result
}

/// Find Amp extension bundles in VS Code / Cursor / Windsurf.
fn find_extension_bundles() -> Vec<PathBuf> {
    let mut result = Vec::new();
    let Ok(home) = byokey_daemon::paths::home_dir() else {
        return result;
    };
    for editor_dir in &[".vscode", ".vscode-insiders", ".cursor", ".windsurf"] {
        let ext_root = home.join(editor_dir).join("extensions");
        if !ext_root.is_dir() {
            continue;
        }
        if let Ok(entries) = std::fs::read_dir(&ext_root) {
            for entry in entries.flatten() {
                let name = entry.file_name();
                let name_str = name.to_string_lossy();
                if !name_str.starts_with("sourcegraph.amp-") {
                    continue;
                }
                if let Ok(walker) = glob_walk(&entry.path()) {
                    for js_file in walker {
                        if let Ok(meta) = js_file.metadata()
                            && meta.len() > 1_000_000
                            && let Ok(real) = std::fs::canonicalize(&js_file)
                            && has_ad_code(&real)
                        {
                            result.push(real);
                        }
                    }
                }
            }
        }
    }
    result
}

/// Recursively yield `.js` files under `dir`.
fn glob_walk(dir: &std::path::Path) -> std::io::Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            if let Ok(sub) = glob_walk(&path) {
                files.extend(sub);
            }
        } else if path.extension().is_some_and(|e| e == "js") {
            files.push(path);
        }
    }
    Ok(files)
}

fn has_ad_code(path: &std::path::Path) -> bool {
    std::fs::read(path)
        .map(|data| {
            data.windows(b"fireImpressionIfNeeded".len())
                .any(|w| w == b"fireImpressionIfNeeded")
        })
        .unwrap_or(false)
}
