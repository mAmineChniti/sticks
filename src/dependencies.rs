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
	if !Path::new("Makefile").exists() {
		return Err(Error::new(
			ErrorKind::NotFound,
			"Makefile not found in the current directory. Cannot remove a dependency.",
		));
	}

	let makefile_path = "Makefile";
	let mut makefile_content = String::new();

	{
		let mut makefile = fs::File::open(makefile_path)?;
		makefile.read_to_string(&mut makefile_content)?;
	}

	let mut updated_makefile_content = String::new();
	let mut found_dependencies = false;
	let install_deps_prefix = "\nsudo apt install -y";
	let mut remaining_deps: Vec<String> = Vec::new();

	for line in makefile_content.lines() {
		if line.trim().starts_with(install_deps_prefix) {
			let existing_deps: Vec<String> = line[install_deps_prefix.len()..]
				.split_whitespace()
				.map(|s| s.to_string())
				.collect();

			remaining_deps = existing_deps
				.into_iter()
				.filter(|dep| !dependency_names.contains(dep))
				.collect();

			if remaining_deps.is_empty() {
				found_dependencies = true;
				continue; // Skip writing this line back (since no deps remain)
			} else {
				let new_install_deps_line =
					format!("{} {}", install_deps_prefix, remaining_deps.join(" "));
				updated_makefile_content.push_str(&new_install_deps_line);
				updated_makefile_content.push('\n');
				found_dependencies = true;
				continue;
			}
		} else {
			updated_makefile_content.push_str(line);
			updated_makefile_content.push('\n');
		}
	}

	if found_dependencies {
		let mut makefile = fs::File::create(makefile_path)?;
		makefile.write_all(updated_makefile_content.as_bytes())?;
		println!("Dependencies {:?} removed from Makefile.", dependency_names);

		// If no dependencies are left after removal, remove install-deps rule and adjust the 'all' rule
		if remaining_deps.is_empty() {
			let mut final_makefile_content = String::new();
			let mut skip_lines = false;
			for line in updated_makefile_content.lines() {
				if skip_lines {
					if line.trim().is_empty() {
						skip_lines = false;
					}
					continue;
				}
				if line.contains("install-deps:") {
					skip_lines = true; // Skip the entire install-deps rule
					continue;
				}
				final_makefile_content.push_str(line);
				final_makefile_content.push('\n');
			}
			if final_makefile_content.contains("all: clean install-deps") {
				final_makefile_content =
					final_makefile_content.replace("all: clean install-deps", "all: clean");
			}
			let mut makefile = fs::File::create(makefile_path)?;
			makefile.write_all(final_makefile_content.as_bytes())?;
			println!("Removed install-deps rule and cleaned up Makefile.");
		}
	} else {
		println!("Dependencies {:?} not found in Makefile.", dependency_names);
	}

	Ok(())
}
