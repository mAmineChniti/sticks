// lib.rs
pub mod dependencies;
mod file_handler;
pub mod languages;
pub mod sources;
pub mod updater;

pub use dependencies::{add_dependency, remove_dependency};
pub use file_handler::create_dir;
pub use languages::{Language, LanguageConsts};
pub use sources::add_sources;
pub use updater::update_project;

use std::io::{Error, ErrorKind, Result};

/// Creates a new project with the given name and language.
pub fn create_project(project_name: &str, language: Language) -> Result<()> {
	let hello_world_content = language.generate_helloworld_content();
	let makefile_content = language.generate_makefile_content(project_name);

	// Ensure the src directory exists
	std::fs::create_dir_all("src")
		.map_err(|e| Error::new(e.kind(), format!("Failed to create src directory: {}", e)))?;

	// Write the hello world content to a file
	std::fs::write(
		format!("src/main.{}", language.extension()),
		hello_world_content,
	)
	.map_err(|e| Error::new(e.kind(), format!("Failed to write hello world file: {}", e)))?;

	// Write the makefile content to a file
	std::fs::write("Makefile", makefile_content)
		.map_err(|e| Error::new(e.kind(), format!("Failed to write Makefile: {}", e)))?;

	Ok(())
}

/// Creates a new project directory and initializes the project.
pub fn new_project(project_name: &str, language: Language) -> Result<()> {
	create_dir(project_name)?;
	create_project(project_name, language)?;
	Ok(())
}

/// Initializes a project in the current directory.
pub fn init_project(language: Language) -> Result<()> {
	let current_dir = std::env::current_dir()?;
	let current_dir_name = current_dir
		.file_name()
		.ok_or_else(|| Error::new(ErrorKind::Other, "Failed to get directory name"))?
		.to_str()
		.ok_or_else(|| Error::new(ErrorKind::Other, "Failed to convert to string"))?;
	create_project(current_dir_name, language)?;
	Ok(())
}
