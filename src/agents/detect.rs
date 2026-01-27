use std::env;
use std::path::PathBuf;
use std::process::Command;

use super::data::{Agent, InstalledInfo};

pub fn detect_installed(agent: &Agent) -> InstalledInfo {
    let binary = match &agent.cli_binary {
        Some(b) => b,
        None => return InstalledInfo::default(),
    };

    // Only detect CLI tools
    if agent.installation_method.as_deref() != Some("cli") {
        return InstalledInfo::default();
    }

    // Try to find the binary
    let path = find_binary(binary);
    if path.is_none() {
        return InstalledInfo::default();
    }

    // Try to get version
    let version = get_version(
        binary,
        &agent.version_command,
        agent.version_regex.as_deref(),
    );

    InstalledInfo {
        version,
        path: path.map(|p| p.to_string_lossy().to_string()),
    }
}

fn find_binary(name: &str) -> Option<PathBuf> {
    // First try which/where
    if let Some(path) = which_binary(name) {
        return Some(path);
    }

    // Check common locations
    let home = env::var("HOME").ok()?;
    let common_paths = [
        format!("/opt/homebrew/bin/{}", name),
        format!("/usr/local/bin/{}", name),
        format!("{}/.local/bin/{}", home, name),
        format!("{}/.cargo/bin/{}", home, name),
        format!("{}/.npm-global/bin/{}", home, name),
        format!("/usr/bin/{}", name),
    ];

    for path_str in common_paths {
        let path = PathBuf::from(&path_str);
        if path.exists() {
            return Some(path);
        }
    }

    None
}

fn which_binary(name: &str) -> Option<PathBuf> {
    let output = Command::new("which").arg(name).output().ok()?;

    if output.status.success() {
        let path_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !path_str.is_empty() {
            return Some(PathBuf::from(path_str));
        }
    }

    None
}

fn get_version(
    binary: &str,
    version_cmd: &[String],
    version_regex: Option<&str>,
) -> Option<String> {
    if version_cmd.is_empty() {
        return None;
    }

    let output = Command::new(binary).args(version_cmd).output().ok()?;

    let output_str = if output.status.success() {
        String::from_utf8_lossy(&output.stdout).to_string()
    } else {
        // Some tools output version to stderr
        String::from_utf8_lossy(&output.stderr).to_string()
    };

    extract_version(&output_str, version_regex)
}

fn extract_version(output: &str, regex_pattern: Option<&str>) -> Option<String> {
    let pattern = regex_pattern.unwrap_or(r"([0-9]+\.[0-9]+\.[0-9]+)");

    // Simple regex-like extraction (avoid regex crate dependency)
    // Look for version pattern in output
    for line in output.lines() {
        if let Some(version) = extract_semver_from_line(line, pattern) {
            return Some(version);
        }
    }
    None
}

fn extract_semver_from_line(line: &str, _pattern: &str) -> Option<String> {
    // Simple extraction: find X.Y.Z pattern
    let chars: Vec<char> = line.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        if chars[i].is_ascii_digit() {
            let start = i;
            let mut dots = 0;

            while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '.') {
                if chars[i] == '.' {
                    dots += 1;
                }
                i += 1;
            }

            if dots >= 2 {
                let version: String = chars[start..i].iter().collect();
                // Validate it looks like semver
                let parts: Vec<&str> = version.split('.').collect();
                if parts.len() >= 3 && parts.iter().all(|p| !p.is_empty()) {
                    return Some(version.trim_end_matches('.').to_string());
                }
            }
        }
        i += 1;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_semver() {
        assert_eq!(
            extract_version("claude-code v1.0.30", None),
            Some("1.0.30".to_string())
        );
        assert_eq!(
            extract_version("aider v0.82.1", None),
            Some("0.82.1".to_string())
        );
        assert_eq!(
            extract_version("Version: 2.3.4-beta", None),
            Some("2.3.4".to_string())
        );
    }

    #[test]
    fn test_no_version() {
        assert_eq!(extract_version("no version here", None), None);
        assert_eq!(extract_version("1.2", None), None); // Not enough parts
    }
}
