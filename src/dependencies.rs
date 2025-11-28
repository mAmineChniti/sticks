use anyhow::{Context, Result};
use std::collections::HashSet;
use std::fs;
use std::path::Path;

use crate::constants::makefile;

pub fn add_dependencies(dependency_names: &[String]) -> Result<()> {
	if !Path::new(makefile::FILENAME).exists() {
		anyhow::bail!("Makefile not found in the current directory");
	}

	let mut makefile_content =
		fs::read_to_string(makefile::FILENAME).context("Failed to read Makefile")?;

	if !makefile_content.contains("all: clean install-deps") {
		makefile_content = makefile_content.replace("all: clean", "all: clean install-deps");
	}

	let install_deps_line =
		if let Some(install_deps_pos) = makefile_content.find(makefile::INSTALL_DEPS_PREFIX) {
			let existing_deps_start = install_deps_pos + makefile::INSTALL_DEPS_PREFIX.len();
			let existing_deps = &makefile_content[existing_deps_start..]
				.lines()
				.next()
				.unwrap_or("")
				.trim();
			existing_deps.to_string()
		} else {
			String::new()
		};

	let mut current_deps: HashSet<String> = install_deps_line
		.split_whitespace()
		.map(|s| s.to_string())
		.collect();

	let mut added_deps = Vec::new();

	for dep in dependency_names {
		if current_deps.insert(dep.clone()) {
			added_deps.push(dep);
		}
	}

	if added_deps.is_empty() {
		println!("All dependencies are already present.");
		return Ok(());
	}

	let mut sorted_deps: Vec<_> = current_deps.into_iter().collect();
	sorted_deps.sort();
	let new_install_deps_line = format!(
		"{} {}",
		makefile::INSTALL_DEPS_PREFIX,
		sorted_deps.join(" ")
	);

	if makefile_content.contains(makefile::INSTALL_DEPS_PREFIX) {
		makefile_content =
			makefile_content.replace(makefile::INSTALL_DEPS_PREFIX, &new_install_deps_line);
	} else {
		makefile_content.push_str(&format!("\ninstall-deps:\n\t{}\n", new_install_deps_line));
	}

	fs::write(makefile::FILENAME, makefile_content).context("Failed to write updated Makefile")?;

	println!("Added dependencies: {:?}", added_deps);

	Ok(())
}

pub fn remove_dependencies(dependency_names: &[String]) -> Result<()> {
	if !Path::new(makefile::FILENAME).exists() {
		anyhow::bail!("Makefile not found in the current directory");
	}

	let makefile_content = fs::read_to_string(makefile::FILENAME)?;
	let lines: Vec<&str> = makefile_content.lines().collect();
	let mut updated_lines: Vec<String> = Vec::new();
	let mut i = 0;
	let mut install_deps_found = false;
	let mut dependencies_removed = false;
	let deps_to_remove: HashSet<&String> = dependency_names.iter().collect();

	while i < lines.len() {
		let line = lines[i];

		if line.starts_with("all:") {
			let parts: Vec<&str> = line.split(':').collect();
			if parts.len() >= 2 {
				let targets: Vec<&str> = parts[1]
					.split_whitespace()
					.filter(|t| *t != "install-deps")
					.collect();
				if !targets.is_empty() {
					updated_lines.push(format!("all: {}", targets.join(" ")));
				} else {
					updated_lines.push("all: clean".to_string());
				}
			} else {
				updated_lines.push(line.to_string());
			}
		} else if line.starts_with("install-deps:") {
			install_deps_found = true;
			if i + 1 < lines.len() {
				let cmd_line = lines[i + 1];
				if cmd_line
					.trim_start()
					.starts_with(makefile::INSTALL_DEPS_PREFIX)
				{
					let deps_str =
						cmd_line.trim_start()[makefile::INSTALL_DEPS_PREFIX.len()..].trim();
					let mut current_deps: HashSet<String> =
						deps_str.split_whitespace().map(String::from).collect();
					let original_len = current_deps.len();
					current_deps.retain(|dep| !deps_to_remove.contains(&dep));
					let removed = original_len - current_deps.len();

					if removed > 0 {
						dependencies_removed = true;
						if !current_deps.is_empty() {
							updated_lines.push("install-deps:".to_string());
							let mut sorted_deps: Vec<_> = current_deps.into_iter().collect();
							sorted_deps.sort();
							updated_lines.push(format!(
								"\t{} {}",
								makefile::INSTALL_DEPS_PREFIX,
								sorted_deps.join(" ")
							));
						} else {
							println!(
								"All specified dependencies removed. Removing install-deps rule."
							);
						}
					} else {
						updated_lines.push(line.to_string());
						if i + 1 < lines.len() {
							updated_lines.push(lines[i + 1].to_string());
						}
					}
				} else {
					updated_lines.push(line.to_string());
				}
				i += 1;
			} else {
				updated_lines.push(line.to_string());
			}
		} else {
			updated_lines.push(line.to_string());
		}

		i += 1;
	}

	if dependencies_removed && install_deps_found {
		println!("Dependencies {:?} removed from Makefile.", dependency_names);
	} else {
		println!("Dependencies {:?} not found in Makefile.", dependency_names);
	}

	fs::write(makefile::FILENAME, updated_lines.join("\n"))
		.context("Failed to write updated Makefile")?;
	Ok(())
}
