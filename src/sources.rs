use crate::languages::source_extension;
use anyhow::{Context, Result};
use std::fs::{self, OpenOptions};
use std::io::{ErrorKind, Write};
use std::path::{Component, Path};

pub fn add_sources(source_names: &[&str]) -> Result<()> {
	if !Path::new("src").is_dir() {
		anyhow::bail!(
			"src directory not found. Cannot add sources and headers.\n\
			Maybe try creating a new project or initializing a new project in the current directory"
		);
	}

	for source_name in source_names {
		validate_source_name(source_name)?;
	}

	let src_path = Path::new("src");
	let include_path = Path::new("include");
	for directory in [src_path, include_path] {
		if let Ok(metadata) = fs::symlink_metadata(directory)
			&& (metadata.file_type().is_symlink() || !metadata.is_dir())
		{
			anyhow::bail!(
				"Project directory must be a real directory: {}",
				directory.display()
			);
		}
	}
	let extension = determine_extension(src_path)?;

	for &source_name in source_names {
		let source_file = format!("{}.{}", source_name, extension);
		let source_path = src_path.join(&source_file);

		if source_path.exists() {
			println!("Source file {} already exists. Skipping.", source_file);
			continue;
		}

		fs::create_dir_all(include_path).context("Failed to create include directory")?;

		match OpenOptions::new()
			.write(true)
			.create_new(true)
			.open(&source_path)
		{
			Ok(mut source) => source
				.write_all(b"")
				.with_context(|| format!("Failed to create source file {}", source_file))?,
			Err(error) if error.kind() == ErrorKind::AlreadyExists => {
				println!("Source file {} already exists. Skipping.", source_file);
				continue;
			}
			Err(error) => {
				return Err(error)
					.with_context(|| format!("Failed to create source file {}", source_file));
			}
		}

		let header_file = format!("{}.h", source_name);
		let header_path = include_path.join(&header_file);
		let guard = header_guard(source_name);
		let header_content = format!("#ifndef {}_H\n#define {}_H\n\n#endif\n", guard, guard);

		let header_result: Result<()> = match OpenOptions::new()
			.write(true)
			.create_new(true)
			.open(&header_path)
		{
			Ok(mut header) => header
				.write_all(header_content.as_bytes())
				.with_context(|| format!("Failed to create header file {}", header_file)),
			Err(error) if error.kind() == ErrorKind::AlreadyExists => Ok(()),
			Err(error) => {
				Err(error).with_context(|| format!("Failed to create header file {}", header_file))
			}
		};
		if let Err(error) = header_result {
			let _ = fs::remove_file(&source_path);
			return Err(error);
		}

		println!("Added source: {}", source_file);
	}

	Ok(())
}

fn validate_source_name(source_name: &str) -> Result<()> {
	if source_name.is_empty() {
		anyhow::bail!("Source name cannot be empty");
	}
	if source_name == "." || source_name == ".." || source_name.starts_with('-') {
		anyhow::bail!("Invalid source name: {}", source_name);
	}
	if source_name.contains('/') || source_name.contains('\\') {
		anyhow::bail!("Invalid source name: {}", source_name);
	}

	let mut components = Path::new(source_name).components();
	if !matches!(components.next(), Some(Component::Normal(_)))
		|| components.next().is_some()
		|| !source_name.chars().all(|character| {
			character.is_ascii_alphanumeric() || matches!(character, '_' | '-' | '.')
		}) {
		anyhow::bail!("Invalid source name: {}", source_name);
	}

	Ok(())
}

fn header_guard(source_name: &str) -> String {
	let mut guard = String::new();
	for character in source_name.chars() {
		if character.is_ascii_alphanumeric() {
			guard.push(character.to_ascii_uppercase());
		} else {
			guard.push_str(match character {
				'_' => "_U",
				'-' => "_H",
				'.' => "_D",
				_ => "_",
			});
		}
	}

	if guard
		.chars()
		.next()
		.is_some_and(|character| character.is_ascii_digit())
	{
		guard.insert(0, '_');
	}

	guard
}

fn determine_extension(src_path: &Path) -> Result<&'static str> {
	match source_extension(src_path)? {
		Some(extension) => Ok(extension),
		None => {
			eprintln!("No existing source files found in src/. Defaulting to .c extension.");
			Ok("c")
		}
	}
}
