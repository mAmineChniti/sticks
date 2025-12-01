pub mod build_systems;
pub mod constants;
pub mod dependencies;
pub mod features;
mod file_handler;
pub mod interactive;
pub mod languages;
pub mod package_managers;
pub mod sources;
pub mod templates;
pub mod updater;

pub use build_systems::{
	get_generator, BuildSystem, BuildSystemGenerator, CMakeGenerator, MakefileGenerator,
};
pub use dependencies::{add_dependencies, remove_dependencies};
pub use features::{
	add_package_manager_to_project, convert_build_system, detect_build_system,
	detect_package_manager, list_features, remove_package_manager_from_project,
};
pub use file_handler::create_dir;
pub use languages::{Language, LanguageConsts};
pub use package_managers::{
	get_package_manager_generator, PackageManager, PackageManagerGenerator,
};
pub use sources::add_sources;
pub use templates::*;
pub use updater::update_project;

use anyhow::{Context, Result};
use std::fs;

pub fn create_project(project_name: &str, language: Language) -> Result<()> {
	create_project_with_system(project_name, language, BuildSystem::Makefile)
}

/// Creates a new project scaffold with the specified name, programming language, and build system,
/// without adding a package manager.
///
/// Returns `Ok(())` on success or an error with context on failure.
///
/// # Examples
///
/// ```
/// let res = create_project_with_system("my_app", Language::Rust, BuildSystem::Makefile);
/// assert!(res.is_ok());
/// ```
pub fn create_project_with_system(
	project_name: &str,
	language: Language,
	build_system: BuildSystem,
) -> Result<()> {
	create_project_with_full_config(project_name, language, build_system, None)
}

/// Creates a new project using the given language, build system, and package manager.
///
/// The function scaffolds project files and, if provided, generates the package manager's manifest.
///
/// # Examples
///
/// ```
/// use project_tooling::{create_project_with_system_and_pm, Language, BuildSystem, PackageManager};
///
/// let res = create_project_with_system_and_pm(
///     "my_project",
///     Language::Rust,
///     BuildSystem::Makefile,
///     PackageManager::Cargo,
/// );
/// assert!(res.is_ok());
/// ```
///
/// # Returns
///
/// `Ok(())` on success, an error otherwise.
pub fn create_project_with_system_and_pm(
	project_name: &str,
	language: Language,
	build_system: BuildSystem,
	package_manager: PackageManager,
) -> Result<()> {
	create_project_with_full_config(project_name, language, build_system, Some(package_manager))
}

/// Creates project scaffolding for `project_name` using the specified language and build system,
/// optionally generating a package manager manifest.
///
/// This writes source files, editor and formatting configs, VS Code launch/settings/tasks,
/// README, and build system files into the current directory. If `package_manager` is `Some`,
/// the corresponding manifest file for that package manager is also generated.
///
/// # Returns
///
/// `Ok(())` on success, or an `Err` with contextual information if any filesystem or template
/// operation fails.
///
/// # Examples
///
/// ```
/// use anyhow::Result;
/// // Create a simple project using Language::C and the Makefile build system without a package manager.
/// let _res: Result<()> = create_project_with_full_config("myapp", Language::C, BuildSystem::Makefile, None);
/// ```
fn create_project_with_full_config(
	project_name: &str,
	language: Language,
	build_system: BuildSystem,
	package_manager: Option<PackageManager>,
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

	if let Some(pm) = package_manager {
		let pm_generator = get_package_manager_generator(pm);
		let manifest = pm_generator.generate_manifest(project_name);
		fs::write(pm_generator.extension(), manifest)
			.with_context(|| format!("Failed to write {} manifest", pm_generator.name()))?;
		println!("📦 Generated {} configuration", pm);
	}

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

/// Initializes the current directory as a new project for the given language using the Makefile build system.
///
/// # Arguments
///
/// * `language` - The programming language to scaffold the project for.
///
/// # Returns
///
/// `Ok(())` on success, or an error with context if project initialization fails.
///
/// # Examples
///
/// ```
/// use crate::Language;
/// let res = crate::init_project(Language::Rust);
/// assert!(res.is_ok());
/// ```
pub fn init_project(language: Language) -> Result<()> {
	init_project_with_system(language, BuildSystem::Makefile)
}

/// Initializes the current directory as a new project using the specified language and build system.

///

/// The project name is derived from the current directory's name. Files and configuration for the

/// language and build system will be created or updated in place.

///

/// # Examples

///

/// ```no_run

/// use crate::{init_project_with_system, Language, BuildSystem};

///

/// // Initialize the current directory as a Rust project using Makefile

/// let _ = init_project_with_system(Language::Rust, BuildSystem::Makefile);

/// ```
pub fn init_project_with_system(language: Language, build_system: BuildSystem) -> Result<()> {
	init_project_with_system_and_pm_internal(language, build_system, None)
}

/// Initializes the current directory as a new project using the given language, build system, and package manager.
///
/// On success this returns `Ok(())`. Any failure during initialization returns an error with context.
///
/// # Examples
///
/// ```
/// use crate::{init_project_with_system_and_pm, Language, BuildSystem, PackageManager};
///
/// // Initialize current directory as a Rust project using Makefile and Cargo.
/// let result = init_project_with_system_and_pm(Language::Rust, BuildSystem::Makefile, PackageManager::Cargo);
/// assert!(result.is_ok());
/// ```
pub fn init_project_with_system_and_pm(
	language: Language,
	build_system: BuildSystem,
	package_manager: PackageManager,
) -> Result<()> {
	init_project_with_system_and_pm_internal(language, build_system, Some(package_manager))
}

/// Initializes the current directory as a new project using the provided language, build system, and optional package manager.
///
/// This uses the current directory name as the project name, creates the project files accordingly, and prints a confirmation on success. Errors encountered while reading the current directory or creating files are returned with context.
///
/// # Examples
///
/// ```no_run
/// use crate::{init_project_with_system_and_pm_internal, Language, BuildSystem, PackageManager};
///
/// // Initialize current directory as a Rust project using the Makefile build system without a package manager
/// let _ = init_project_with_system_and_pm_internal(Language::Rust, BuildSystem::Makefile, None);
/// ```
fn init_project_with_system_and_pm_internal(
	language: Language,
	build_system: BuildSystem,
	package_manager: Option<PackageManager>,
) -> Result<()> {
	let current_dir = std::env::current_dir().context("Failed to get current directory")?;
	let current_dir_name = current_dir
		.file_name()
		.context("Failed to get directory name")?
		.to_str()
		.context("Failed to convert directory name to string")?;
	create_project_with_full_config(current_dir_name, language, build_system, package_manager)?;
	println!("✓ Initialized {} project in current directory", language);
	Ok(())
}