pub mod dependencies;
mod file_handler;
pub mod languages;
pub mod sources;
pub mod updater;

pub use dependencies::{add_dependencies, remove_dependencies};
pub use file_handler::create_dir;
pub use languages::{Language, LanguageConsts};
pub use sources::add_sources;
pub use updater::update_project;

use anyhow::{Context, Result};

pub fn create_project(project_name: &str, language: Language) -> Result<()> {
	let hello_world_content = language.generate_helloworld_content();
	let makefile_content = language.generate_makefile_content(project_name);

	std::fs::create_dir_all("src").context("Failed to create src directory")?;

	std::fs::write(
		format!("src/main.{}", language.extension()),
		hello_world_content,
	)
	.context("Failed to write hello world file")?;

	std::fs::write("Makefile", makefile_content).context("Failed to write Makefile")?;

	println!("✓ Created {} project: {}", language, project_name);

	Ok(())
}

pub fn new_project(project_name: &str, language: Language) -> Result<()> {
	create_dir(project_name)?;
	create_project(project_name, language)?;
	Ok(())
}

pub fn init_project(language: Language) -> Result<()> {
	let current_dir = std::env::current_dir().context("Failed to get current directory")?;
	let current_dir_name = current_dir
		.file_name()
		.context("Failed to get directory name")?
		.to_str()
		.context("Failed to convert directory name to string")?;
	create_project(current_dir_name, language)?;
	println!("✓ Initialized {} project in current directory", language);
	Ok(())
}
