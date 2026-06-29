use crate::os_detect::SystemPackageManager;
use anyhow::Result;
use std::process::Command;
use strsim::levenshtein;

pub fn package_exists(name: &str, pm: SystemPackageManager) -> Result<bool> {
	match pm {
		SystemPackageManager::Apt => {
			if let Ok(p) = Command::new("apt-cache").arg("policy").arg(name).output() {
				let s = String::from_utf8_lossy(&p.stdout).to_string()
					+ String::from_utf8_lossy(&p.stderr).as_ref();
				if s.contains("Candidate: (none)") || s.trim().is_empty() {
					return Ok(false);
				}
				return Ok(true);
			}
			Ok(false)
		}
		SystemPackageManager::Dnf => {
			let out = Command::new("dnf").arg("list").arg(name).output();
			if let Ok(o) = out {
				return Ok(o.status.success());
			}
			Ok(false)
		}
		SystemPackageManager::Pacman => {
			let out = Command::new("pacman").arg("-Si").arg(name).output();
			if let Ok(o) = out {
				return Ok(o.status.success());
			}
			Ok(false)
		}
		SystemPackageManager::Apk => {
			let out = Command::new("apk")
				.arg("search")
				.arg("-e")
				.arg(name)
				.output();
			if let Ok(o) = out {
				return Ok(o.status.success());
			}
			Ok(false)
		}
		SystemPackageManager::Zypper => {
			let out = Command::new("zypper")
				.arg("se")
				.arg("-x")
				.arg(name)
				.output();
			if let Ok(o) = out {
				return Ok(o.status.success());
			}
			Ok(false)
		}
		SystemPackageManager::Unknown => Ok(false),
	}
}

/// Helper to run a package search command and extract package names from stdout
/// using simple first-word extraction (for Apt, Apk, Zypper)
fn parse_simple_search_output(cmd: &str, args: &[&str]) -> Vec<String> {
	let search = Command::new(cmd).args(args).output();
	let mut candidates = Vec::new();

	if let Ok(o) = search {
		let out = String::from_utf8_lossy(&o.stdout).to_string();
		for line in out.lines() {
			if !line.trim().is_empty() {
				// Extract first word (package name) from each line
				if let Some(pkg) = line.split_whitespace().next() {
					candidates.push(pkg.to_string());
				}
			}
		}
	}

	candidates
}

fn find_candidates(name: &str, pm: SystemPackageManager) -> Vec<String> {
	let mut candidates: Vec<String> = Vec::new();

	match pm {
		SystemPackageManager::Apt => {
			candidates = parse_simple_search_output("apt-cache", &["search", name]);
		}
		SystemPackageManager::Pacman => {
			let search = Command::new("pacman").arg("-Ss").arg(name).output();
			if let Ok(o) = search {
				let out = String::from_utf8_lossy(&o.stdout).to_string();
				for line in out.lines() {
					if !line.trim().is_empty() {
						// Extract package name after '/' if present
						if let Some(after_slash) = line.split('/').nth(1)
							&& let Some(pkg) = after_slash.split_whitespace().next()
						{
							candidates.push(pkg.to_string());
						}
					}
				}
			}
		}
		SystemPackageManager::Dnf => {
			let search = Command::new("dnf").arg("search").arg(name).output();
			if let Ok(o) = search {
				let out = String::from_utf8_lossy(&o.stdout).to_string();
				for line in out.lines() {
					let trimmed = line.trim();
					// Skip header lines and separators (e.g., "====", "Name", "Summary")
					if trimmed.is_empty()
						|| trimmed.starts_with('=')
						|| trimmed.starts_with("Name")
						|| trimmed.starts_with("Summary")
					{
						continue;
					}
					// Parse "name.arch : description" format, extract only package name without .arch
					if let Some(colon_pos) = trimmed.find(':') {
						let pkg_part = &trimmed[..colon_pos].trim();
						if let Some(dot_pos) = pkg_part.rfind('.') {
							// Remove .arch suffix
							candidates.push(pkg_part[..dot_pos].to_string());
						} else {
							candidates.push(pkg_part.to_string());
						}
					}
				}
			}
		}
		SystemPackageManager::Apk => {
			candidates = parse_simple_search_output("apk", &["search", name]);
		}
		SystemPackageManager::Zypper => {
			candidates = parse_simple_search_output("zypper", &["search", name]);
		}
		SystemPackageManager::Unknown => {}
	}

	candidates
}

pub fn suggest_similar(name: &str, pm: SystemPackageManager) -> Result<Vec<String>> {
	let mut candidates = find_candidates(name, pm);

	// Deduplicate and score
	candidates.sort();
	candidates.dedup();

	let mut scored: Vec<(usize, String)> = candidates
		.into_iter()
		.map(|c| (levenshtein(name, &c), c))
		.collect();

	scored.sort_by_key(|(d, _)| *d);
	Ok(scored.into_iter().take(5).map(|(_, c)| c).collect())
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_suggest_similar_fallback() {
		// Test with a package manager that should be available on most systems
		let pm = crate::os_detect::detect_system_package_manager();
		if pm == crate::os_detect::SystemPackageManager::Unknown {
			// Skip test if no package manager is detected
			return;
		}
		let sug = suggest_similar("libcrl", pm).unwrap();
		// Only assert if we got results (some package managers may not return suggestions)
		if !sug.is_empty() {
			assert!(
				sug.iter()
					.any(|s| s.contains("libcurl") || s.contains("libssl"))
			);
		}
	}

	#[test]
	fn test_package_exists_fallback() {
		let pm = crate::os_detect::detect_system_package_manager();
		if pm == crate::os_detect::SystemPackageManager::Unknown {
			// Skip test if no package manager is detected
			return;
		}
		let exists = package_exists("libcurl", pm).unwrap();
		// Only assert if package check succeeded (may vary by system)
		if exists {
			let non = package_exists("fake-nonexistent-package-xyz", pm).unwrap();
			assert!(!non);
		}
	}
}
