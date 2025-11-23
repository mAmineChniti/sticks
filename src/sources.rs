use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

pub fn add_sources(source_names: &[&str]) -> Result<()> {
	if !Path::new("src").exists() {
		anyhow::bail!(
			"src directory not found. Cannot add sources and headers.\n\
			Maybe try creating a new project or initializing a new project in the current directory"
		);
	}

	let src_path = Path::new("src");

	let extension = determine_extension(src_path)?;

	for &source_name in source_names {
		let source_file = format!("{}.{}", source_name, extension);
		let source_path = src_path.join(&source_file);

		if source_path.exists() {
			println!("Source file {} already exists. Skipping.", source_file);
		} else {
			fs::write(&source_path, "")
				.with_context(|| format!("Failed to create source file {}", source_file))?;

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
			)
			.with_context(|| format!("Failed to create header file {}", header_file))?;

			println!("Added source: {}", source_file);
		}
	}

	Ok(())
}

fn determine_extension(src_path: &Path) -> Result<&'static str> {
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
			Ok("c")
		}
	}
}
