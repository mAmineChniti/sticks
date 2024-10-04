// dependencies.rs
use std::fs::{self, File, OpenOptions};
use std::io::{Error, ErrorKind, Read, Result, Write};
use std::path::Path;

pub fn add_dependencies(dependency_names: &Vec<String>) -> Result<()> {
	if !Path::new("Makefile").exists() {
		return Err(Error::new(
			ErrorKind::NotFound,
			"Makefile not found in the current directory. Cannot add dependencies.",
		));
	}

	let mut makefile_content = String::new();
	let mut makefile = File::open("Makefile")?;
	makefile.read_to_string(&mut makefile_content)?;

	if !makefile_content.contains("all: clean install-deps") {
		makefile_content = makefile_content.replace("all: clean", "all: clean install-deps");
	}

	let install_deps_prefix = "sudo apt install -y";

	let install_deps_line =
		if let Some(install_deps_pos) = makefile_content.find(install_deps_prefix) {
			let existing_deps_start = install_deps_pos + install_deps_prefix.len();
			let existing_deps = &makefile_content[existing_deps_start..]
				.lines()
				.next()
				.unwrap_or("")
				.trim();
			existing_deps.to_string()
		} else {
			String::new()
		};

	let mut current_deps: Vec<String> = install_deps_line
		.split_whitespace()
		.map(|s| s.to_string())
		.collect();

	let mut added_deps = Vec::new();

	for dep in dependency_names {
		if !current_deps.contains(dep) {
			current_deps.push(dep.clone());
			added_deps.push(dep);
		}
	}

	if added_deps.is_empty() {
		println!("All dependencies are already present.");
		return Ok(());
	}

	let new_install_deps_line = format!("{} {}", install_deps_prefix, current_deps.join(" "));

	if makefile_content.contains(install_deps_prefix) {
		makefile_content = makefile_content.replace(install_deps_prefix, &new_install_deps_line);
	} else {
		makefile_content.push_str(&format!("\ninstall-deps:\n\t{}\n", new_install_deps_line));
	}

	let mut makefile = OpenOptions::new()
		.write(true)
		.truncate(true)
		.create(true)
		.open("Makefile")?;
	makefile.write_all(makefile_content.as_bytes())?;

	println!("Added dependencies: {:?}", added_deps);

	Ok(())
}

pub fn remove_dependencies(dependency_names: &Vec<String>) -> Result<()> {
	const MAKEFILE: &str = "Makefile";
	const INSTALL_DEPS_PREFIX: &str = "sudo apt install -y";

	if !Path::new(MAKEFILE).exists() {
		return Err(Error::new(
			ErrorKind::NotFound,
			"Makefile not found in the current directory. Cannot remove dependencies.",
		));
	}

	let makefile_content = fs::read_to_string(MAKEFILE)?;
	let lines: Vec<&str> = makefile_content.lines().collect();
	let mut updated_lines: Vec<String> = Vec::new();
	let mut i = 0;
	let mut install_deps_found = false;
	let mut dependencies_removed = false;

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
				if cmd_line.trim_start().starts_with(INSTALL_DEPS_PREFIX) {
					let deps_str = cmd_line.trim_start()[INSTALL_DEPS_PREFIX.len()..].trim();
					let mut current_deps: Vec<String> =
						deps_str.split_whitespace().map(String::from).collect();
					let original_len = current_deps.len();
					current_deps.retain(|dep| !dependency_names.contains(dep));
					let removed = original_len - current_deps.len();

					if removed > 0 {
						dependencies_removed = true;
						if !current_deps.is_empty() {
							updated_lines.push("install-deps:".to_string());
							updated_lines.push(format!(
								"\t{} {}",
								INSTALL_DEPS_PREFIX,
								current_deps.join(" ")
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
				i += 1; // Skip the command line
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

	fs::write(MAKEFILE, updated_lines.join("\n"))?;
	Ok(())
}
