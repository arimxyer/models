use std::path::PathBuf;
use std::process::Command;

use super::data::{Agent, InstalledInfo};

pub fn detect_installed(agent: &Agent) -> InstalledInfo {
    // Detect any agent with a cli_binary (CLI tools, IDEs with launchers, etc.)
    let binary = match &agent.cli_binary {
        Some(b) => b,
        None => return InstalledInfo::default(),
    };

    // Try to get version directly - this also confirms the binary exists
    // Skip the separate `which` call since --version tells us if it's installed
    let (version, path) = get_version_and_path(
        binary,
        &agent.version_command,
        agent.version_regex.as_deref(),
    );

    if version.is_none() && path.is_none() {
        return InstalledInfo::default();
    }

    InstalledInfo { version, path }
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

/// Get version and path in one operation - avoids separate `which` call
fn get_version_and_path(
    binary: &str,
    version_cmd: &[String],
    version_regex: Option<&str>,
) -> (Option<String>, Option<String>) {
    if version_cmd.is_empty() {
        return (None, None);
    }

    // Try to run the version command - if it works, the binary exists
    let output = match Command::new(binary).args(version_cmd).output() {
        Ok(o) => o,
        Err(_) => return (None, None), // Binary not found or not executable
    };

    let output_str = if output.status.success() {
        String::from_utf8_lossy(&output.stdout).to_string()
    } else {
        // Some tools output version to stderr
        String::from_utf8_lossy(&output.stderr).to_string()
    };

    let version = extract_version(&output_str, version_regex);

    // Only look up path if we found a version (binary definitely exists)
    let path = if version.is_some() {
        which_binary(binary).map(|p| p.to_string_lossy().to_string())
    } else {
        None
    };

    (version, path)
}

fn extract_version(output: &str, regex_pattern: Option<&str>) -> Option<String> {
    if let Some(pattern) = regex_pattern {
        // Use the provided regex pattern — expects a capture group for the version
        if let Ok(re) = regex::Regex::new(pattern) {
            for line in output.lines() {
                if let Some(captures) = re.captures(line) {
                    // Return first capture group if present, otherwise full match
                    let version = captures
                        .get(1)
                        .or_else(|| captures.get(0))
                        .map(|m| m.as_str().to_string());
                    if version.is_some() {
                        return version;
                    }
                }
            }
        }
    }

    // Default: find X.Y.Z semver pattern
    let re = regex::Regex::new(r"(\d+\.\d+\.\d+)").ok()?;
    for line in output.lines() {
        if let Some(captures) = re.captures(line) {
            if let Some(m) = captures.get(1) {
                return Some(m.as_str().to_string());
            }
        }
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
