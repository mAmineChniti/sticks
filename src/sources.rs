// sources.rs
use std::fs;
use std::io::{Error, ErrorKind, Result};
use std::path::Path;

/// Adds source files and their headers to the project.
pub fn add_sources(source_names: &[&str]) -> Result<()> {
	if !Path::new("src").exists() {
		eprintln!("src directory not found. Cannot add sources and headers.");
		eprintln!("Maybe try creating a new project or initializing a new project in the current directory");
		return Err(Error::new(ErrorKind::NotFound, "'src' directory not found"));
	}

	let src_path = Path::new("src");

	// Determine the extension based on existing files in src/
	let extension = determine_extension(src_path)?;

	for &source_name in source_names {
		let source_file = format!("{}.{}", source_name, extension);
		let source_path = src_path.join(&source_file);

		// Check if the source file already exists
		if source_path.exists() {
			println!("Source file {} already exists. Skipping.", source_file);
		} else {
			// Create the source file
			fs::write(&source_path, format!("// Code for {}\n", source_name))?;

			// Create corresponding .h file
			let header_file = format!("{}.h", source_name);
			let header_path = src_path.join(&header_file);
			fs::write(
				&header_path,
				format!(
					"#ifndef {}_H\n#define {}_H\n#endif /* {}_H */",
					source_name.to_uppercase(),
					source_name.to_uppercase(),
					source_name.to_uppercase()
				),
			)?;

			println!("Added source: {}", source_file);
		}
	}

	Ok(())
}

/// Determines the file extension based on existing source files in `src/`.
fn determine_extension(src_path: &Path) -> Result<&'static str> {
	// Find the first source file in src/ to determine the extension
	let source_file = fs::read_dir(src_path)?
		.filter_map(|entry| {
			let entry = entry.ok()?;
			let path = entry.path();
			if path.is_file() {
				path.extension()
					.map(|ext| ext.to_string_lossy().to_string())
			} else {
				None
			}
		})
		.next();

	match source_file.as_deref() {
		Some("c") => Ok("c"),
		Some("cpp") => Ok("cpp"),
		_ => {
			eprintln!("No existing source files found in src/. Defaulting to .c extension.");
			Ok("c") // Default to .c if no existing source files are found
		}
	}
}
