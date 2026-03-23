use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};

use crate::config::Config;

pub fn run(dir: Option<PathBuf>, remove: bool, status: bool, config: &Config) -> Result<()> {
    let binary = std::env::current_exe().context("could not determine binary path")?;

    let target_dir = match dir {
        Some(d) => {
            if !d.is_dir() {
                bail!("directory does not exist: {}", d.display());
            }
            d
        }
        None => binary
            .parent()
            .context("could not determine binary directory")?
            .to_path_buf(),
    };

    if status {
        show_status(&binary, &target_dir, config)
    } else if remove {
        remove_links(&binary, &target_dir, config)
    } else {
        create_links(&binary, &target_dir, config)
    }
}

fn show_status(binary: &Path, target_dir: &Path, config: &Config) -> Result<()> {
    println!("Binary: {}", binary.display());
    println!("Directory: {}\n", target_dir.display());

    for (alias, _kind) in config.alias_names() {
        let link_path = target_dir.join(alias);

        match std::fs::read_link(&link_path) {
            Ok(target) => {
                if target == binary {
                    println!("  {alias} -> linked (ok)");
                } else {
                    println!(
                        "  {alias} -> linked to {} (different binary)",
                        target.display()
                    );
                }
            }
            Err(_) => {
                if link_path.exists() {
                    println!("  {alias} -> exists (not a symlink)");
                } else {
                    println!("  {alias} -> not found");
                }
            }
        }
    }

    Ok(())
}

fn create_links(binary: &Path, target_dir: &Path, config: &Config) -> Result<()> {
    let mut created = 0;

    for (alias, _kind) in config.alias_names() {
        let link_path = target_dir.join(alias);

        if link_path.exists() || link_path.symlink_metadata().is_ok() {
            // Check if it already points to our binary
            if let Ok(target) = std::fs::read_link(&link_path) {
                if target == binary {
                    println!("  {alias} -> already linked");
                    continue;
                }
                println!(
                    "  {alias} -> skipped (symlink exists, points to {})",
                    target.display()
                );
            } else {
                println!("  {alias} -> skipped (file already exists)");
            }
            continue;
        }

        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(binary, &link_path)
                .with_context(|| format!("failed to create symlink: {}", link_path.display()))?;
        }

        #[cfg(not(unix))]
        {
            bail!("symlink creation is only supported on Unix systems");
        }

        println!("  {alias} -> {}", link_path.display());
        created += 1;
    }

    if created > 0 {
        println!("\nCreated {created} symlink(s) in {}", target_dir.display());
    } else {
        println!("\nNo new symlinks created.");
    }
    println!("Ensure {} is in your PATH.", target_dir.display());

    Ok(())
}

fn remove_links(binary: &Path, target_dir: &Path, config: &Config) -> Result<()> {
    let mut removed = 0;

    for (alias, _kind) in config.alias_names() {
        let link_path = target_dir.join(alias);

        match std::fs::read_link(&link_path) {
            Ok(target) => {
                if target != binary {
                    println!(
                        "  {alias} -> skipped (points to {}, not our binary)",
                        target.display()
                    );
                    continue;
                }
                std::fs::remove_file(&link_path).with_context(|| {
                    format!("failed to remove symlink: {}", link_path.display())
                })?;
                println!("  {alias} -> removed");
                removed += 1;
            }
            Err(_) => {
                if link_path.exists() {
                    println!("  {alias} -> skipped (not a symlink)");
                } else {
                    println!("  {alias} -> not found");
                }
            }
        }
    }

    println!("\nRemoved {removed} symlink(s).");
    Ok(())
}
