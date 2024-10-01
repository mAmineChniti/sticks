// dependencies.rs
use std::fs::{self, File, OpenOptions};
use std::io::{Error, ErrorKind, Read, Result, Write};
use std::path::Path;

/// Adds a dependency to the Makefile.
pub fn add_dependency(dependency_name: &str) -> Result<()> {
	if !Path::new("Makefile").exists() {
		return Err(Error::new(
			ErrorKind::NotFound,
			"Makefile not found in the current directory. Cannot add a dependency.",
		));
	}

	// Read Makefile content
	let mut makefile_content = String::new();
	let mut makefile = File::open("Makefile")?;
	makefile.read_to_string(&mut makefile_content)?;

	// Check if "all: clean install-deps" is present
	if !makefile_content.contains("all: clean install-deps") {
		// Replace "all: clean" with "all: clean install-deps"
		makefile_content = makefile_content.replace("all: clean", "all: clean install-deps");
	}

	// Check if the dependency is already present in the install-deps rule
	if makefile_content.contains(&format!("sudo apt install -y {}", dependency_name)) {
		println!(
			"Dependency '{}' is already present in the install-deps rule.",
			dependency_name
		);
		return Ok(());
	}

	// Check if "install-deps:" is present
	if !makefile_content.contains("install-deps:") {
		// Add a new install-deps rule
		makefile_content.push_str(&format!(
			"\ninstall-deps:\n\tsudo apt install -y {}\n",
			dependency_name
		));
	} else {
		// Append the dependency to the existing install-deps rule
		makefile_content = makefile_content.replace(
			"sudo apt install -y",
			&format!("sudo apt install -y {}", dependency_name),
		);
	}

	// Write the updated content back to the Makefile
	let mut makefile = OpenOptions::new()
		.write(true)
		.truncate(true)
		.create(true)
		.open("Makefile")?;
	makefile.write_all(makefile_content.as_bytes())?;

	Ok(())
}

/// Removes dependencies from the Makefile.
pub fn remove_dependency(dependency_names: &[&str]) -> Result<()> {
	if !Path::new("Makefile").exists() {
		return Err(Error::new(
			ErrorKind::NotFound,
			"Makefile not found in the current directory. Cannot remove a dependency.",
		));
	}

	let makefile_path = "Makefile";
	let mut makefile_content = String::new();

	// Read the existing Makefile content
	{
		let mut makefile = fs::File::open(makefile_path)?;
		makefile.read_to_string(&mut makefile_content)?;
	}

	let mut updated_makefile_content = String::new();
	let mut found_dependencies = false;

	// Remove the lines containing the dependencies
	for line in makefile_content.lines() {
		if !dependency_names.iter().any(|dep| line.contains(dep)) {
			updated_makefile_content.push_str(line);
			updated_makefile_content.push('\n');
		} else {
			found_dependencies = true;
		}
	}

	if found_dependencies {
		// Write the updated content back to the Makefile
		let mut makefile = fs::File::create(makefile_path)?;
		makefile.write_all(updated_makefile_content.as_bytes())?;
		println!("Dependencies {:?} removed from Makefile.", dependency_names);

		// Check if the install-deps rule is present and there are no more dependencies
		if has_install_deps_rule(&updated_makefile_content)
			&& !updated_makefile_content.contains("sudo apt install -y")
		{
			// Remove the install-deps rule
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
					skip_lines = true;
					continue;
				}
				final_makefile_content.push_str(line);
				final_makefile_content.push('\n');
			}
			if final_makefile_content.contains("all: clean install-deps") {
				final_makefile_content =
					final_makefile_content.replace("all: clean install-deps", "all: clean");
			}
			// Write the updated content back to the Makefile
			let mut makefile = fs::File::create(makefile_path)?;
			makefile.write_all(final_makefile_content.as_bytes())?;
			println!("Removed install-deps rule from Makefile.");
		}
	} else {
		println!("Dependencies {:?} not found in Makefile.", dependency_names);
	}

	Ok(())
}

fn has_install_deps_rule(makefile_content: &str) -> bool {
	makefile_content.contains("install-deps:")
}
