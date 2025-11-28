pub mod build_systems;
pub mod constants;
pub mod dependencies;
mod file_handler;
pub mod languages;
pub mod sources;
pub mod templates;
pub mod updater;

pub use build_systems::{
	get_generator, BuildSystem, BuildSystemGenerator, CMakeGenerator, MakefileGenerator,
};
pub use dependencies::{add_dependencies, remove_dependencies};
pub use file_handler::create_dir;
pub use languages::{Language, LanguageConsts};
pub use sources::add_sources;
pub use templates::*;
pub use updater::update_project;

use anyhow::{Context, Result};
use std::fs;

pub fn create_project(project_name: &str, language: Language) -> Result<()> {
	create_project_with_system(project_name, language, BuildSystem::Makefile)
}

pub fn create_project_with_system(
	project_name: &str,
	language: Language,
	build_system: BuildSystem,
) -> Result<()> {
	let hello_world_content = language.generate_helloworld_content();
	let generator = get_generator(build_system);
	let build_file_content = generator.generate_build_file(language, project_name);

	fs::create_dir_all("src").context("Failed to create src directory")?;

	fs::write(
		format!("src/main.{}", language.extension()),
		hello_world_content,
	)
	.context("Failed to write hello world file")?;

	fs::write(generator.extension(), build_file_content).context("Failed to write build file")?;

	fs::write(".gitignore", templates::generate_gitignore(language))
		.context("Failed to write .gitignore")?;

	fs::write(".editorconfig", templates::generate_editorconfig())
		.context("Failed to write .editorconfig")?;

	fs::write(
		".clang-format",
		templates::generate_clang_format_config(language),
	)
	.context("Failed to write .clang-format")?;

	fs::create_dir_all(".vscode").context("Failed to create .vscode directory")?;

	fs::write(
		".vscode/settings.json",
		templates::generate_vscode_settings(language),
	)
	.context("Failed to write VSCode settings")?;

	fs::write(
		".vscode/launch.json",
		templates::generate_vscode_launch_config(project_name),
	)
	.context("Failed to write VSCode launch config")?;

	fs::write(
		".vscode/tasks.json",
		templates::generate_vscode_tasks_config(),
	)
	.context("Failed to write VSCode tasks")?;

	fs::write(
		"README.md",
		templates::generate_readme(project_name, language),
	)
	.context("Failed to write README")?;

	fs::write(".gitattributes", templates::generate_gitattributes())
		.context("Failed to write .gitattributes")?;

	println!(
		"✓ Created {} project: {} with {}",
		language, project_name, build_system
	);

	Ok(())
}

pub fn new_project(project_name: &str, language: Language) -> Result<()> {
	new_project_with_system(project_name, language, BuildSystem::Makefile)
}

pub fn new_project_with_system(
	project_name: &str,
	language: Language,
	build_system: BuildSystem,
) -> Result<()> {
	create_dir(project_name)?;
	create_project_with_system(project_name, language, build_system)?;
	Ok(())
}

pub fn init_project(language: Language) -> Result<()> {
	init_project_with_system(language, BuildSystem::Makefile)
}

pub fn init_project_with_system(language: Language, build_system: BuildSystem) -> Result<()> {
	let current_dir = std::env::current_dir().context("Failed to get current directory")?;
	let current_dir_name = current_dir
		.file_name()
		.context("Failed to get directory name")?
		.to_str()
		.context("Failed to convert directory name to string")?;
	create_project_with_system(current_dir_name, language, build_system)?;
	println!("✓ Initialized {} project in current directory", language);
	Ok(())
}
