// src/agents/changelog_parser.rs

#[derive(Debug, Clone)]
pub struct Section {
    pub name: String,
    pub changes: Vec<String>,
}

/// Parse a GitHub release body (markdown) into sections and ungrouped changes.
pub fn parse_release_body(body: &str) -> (Vec<Section>, Vec<String>) {
    let skip_headers = ["What's Changed", "Changelog", "Full Changelog"];

    let mut sections: Vec<Section> = Vec::new();
    let mut ungrouped: Vec<String> = Vec::new();
    let mut current_section: Option<Section> = None;

    for line in body.lines() {
        let trimmed = line.trim();

        // Check for markdown headers (##, ###)
        if let Some(header_name) = extract_header(trimmed) {
            // Flush previous section
            if let Some(sec) = current_section.take() {
                if !sec.changes.is_empty() {
                    sections.push(sec);
                }
            }

            // Skip wrapper headers
            if skip_headers.iter().any(|h| header_name.starts_with(h)) {
                continue;
            }

            current_section = Some(Section {
                name: header_name.to_string(),
                changes: Vec::new(),
            });
        } else if let Some(change) = extract_change(trimmed) {
            if let Some(ref mut sec) = current_section {
                sec.changes.push(change);
            } else {
                ungrouped.push(change);
            }
        }
    }

    // Flush last section
    if let Some(sec) = current_section {
        if !sec.changes.is_empty() {
            sections.push(sec);
        }
    }

    (sections, ungrouped)
}

fn extract_header(line: &str) -> Option<&str> {
    // Match ## or ### headers (not #, which is usually the title)
    let stripped = line
        .strip_prefix("### ")
        .or_else(|| line.strip_prefix("## "));
    stripped.map(|s| s.trim())
}

fn extract_change(line: &str) -> Option<String> {
    let stripped = line
        .strip_prefix("- ")
        .or_else(|| line.strip_prefix("* "))
        .or_else(|| line.strip_prefix("+ "));
    stripped.map(|s| s.trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sectioned_changelog() {
        let body = "\
## Bug Fixes
- Fixed crash on startup
- Fixed memory leak

## Features
- Added dark mode
- Added export to CSV";

        let (sections, ungrouped) = parse_release_body(body);
        assert!(ungrouped.is_empty());
        assert_eq!(sections.len(), 2);
        assert_eq!(sections[0].name, "Bug Fixes");
        assert_eq!(sections[0].changes.len(), 2);
        assert_eq!(sections[0].changes[0], "Fixed crash on startup");
        assert_eq!(sections[0].changes[1], "Fixed memory leak");
        assert_eq!(sections[1].name, "Features");
        assert_eq!(sections[1].changes.len(), 2);
        assert_eq!(sections[1].changes[0], "Added dark mode");
        assert_eq!(sections[1].changes[1], "Added export to CSV");
    }

    #[test]
    fn ungrouped_changes() {
        let body = "\
- Fixed crash on startup
- Added dark mode
- Improved performance";

        let (sections, ungrouped) = parse_release_body(body);
        assert!(sections.is_empty());
        assert_eq!(ungrouped.len(), 3);
        assert_eq!(ungrouped[0], "Fixed crash on startup");
        assert_eq!(ungrouped[1], "Added dark mode");
        assert_eq!(ungrouped[2], "Improved performance");
    }

    #[test]
    fn skip_whats_changed_header() {
        let body = "\
## What's Changed
- Fixed crash on startup
- Added dark mode";

        let (sections, ungrouped) = parse_release_body(body);
        assert!(sections.is_empty());
        assert_eq!(ungrouped.len(), 2);
        assert_eq!(ungrouped[0], "Fixed crash on startup");
        assert_eq!(ungrouped[1], "Added dark mode");
    }

    #[test]
    fn asterisk_bullets() {
        let body = "\
## Changes
* Fixed crash on startup
* Added dark mode";

        let (sections, ungrouped) = parse_release_body(body);
        assert!(ungrouped.is_empty());
        assert_eq!(sections.len(), 1);
        assert_eq!(sections[0].changes.len(), 2);
        assert_eq!(sections[0].changes[0], "Fixed crash on startup");
        assert_eq!(sections[0].changes[1], "Added dark mode");
    }

    #[test]
    fn empty_body() {
        let (sections, ungrouped) = parse_release_body("");
        assert!(sections.is_empty());
        assert!(ungrouped.is_empty());
    }

    #[test]
    fn mixed_sections_and_ungrouped() {
        let body = "\
- Ungrouped item 1
- Ungrouped item 2

## Bug Fixes
- Fixed crash on startup

## Features
- Added dark mode";

        let (sections, ungrouped) = parse_release_body(body);
        assert_eq!(ungrouped.len(), 2);
        assert_eq!(ungrouped[0], "Ungrouped item 1");
        assert_eq!(ungrouped[1], "Ungrouped item 2");
        assert_eq!(sections.len(), 2);
        assert_eq!(sections[0].name, "Bug Fixes");
        assert_eq!(sections[0].changes[0], "Fixed crash on startup");
        assert_eq!(sections[1].name, "Features");
        assert_eq!(sections[1].changes[0], "Added dark mode");
    }
}
