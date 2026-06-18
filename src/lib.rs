pub mod build_config;
pub mod build_systems;
pub mod ci_cd;
pub mod cmake_dependencies;
pub mod constants;
pub mod dependencies;
pub mod dependency_manager;
pub mod docs;
pub mod features;
mod file_handler;
pub mod interactive;
pub mod languages;
pub mod makefile_parser;
pub mod multi_target;
pub mod os_detect;
pub mod package_checker;
pub mod package_managers;
pub mod sources;
pub mod static_analysis;
pub mod templates;
pub mod test_framework;
pub mod updater;

pub use build_config::{
	BuildConfig, CompilerFlags, CppStandard, IncludeDirs, LibraryDirs, PreprocessorDefs,
};
pub use build_systems::{
	BuildSystem, BuildSystemGenerator, CMakeGenerator, MakefileGenerator, get_generator,
};
pub use ci_cd::{CiCdGenerator, CiPlatform};
pub use cmake_dependencies::CMakeDependencyManager;
pub use dependencies::{add_dependencies, remove_dependencies};
pub use dependency_manager::DependencyManager;
pub use docs::{DocGenerator, DocTool};
pub use features::{
	add_package_manager_to_project, convert_build_system, convert_build_system_interactive,
	detect_build_file, detect_build_system, detect_package_manager, list_features,
	remove_package_manager_from_project,
};
pub use file_handler::{
	create_dir, project_path, validate_project_name, write_atomic, write_new_file,
};
pub use languages::{Language, LanguageConsts};
pub use makefile_parser::Makefile;
pub use multi_target::{BuildTarget, MultiTargetManager, TargetType};
pub use package_managers::{
	PackageManager, PackageManagerGenerator, get_package_manager_generator,
};
pub use sources::add_sources;
pub use static_analysis::{StaticAnalysisGenerator, StaticAnalysisTool};
pub use templates::*;
pub use test_framework::{TestFramework, TestFrameworkManager};
pub use updater::update_project;

use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

pub fn create_project(project_name: &str, language: Language) -> Result<()> {
	create_project_with_system(project_name, language, BuildSystem::Makefile)
}

pub fn create_project_with_system(
	project_name: &str,
	language: Language,
	build_system: BuildSystem,
) -> Result<()> {
	let root = std::env::current_dir().context("Failed to get current directory")?;
	create_project_in(&root, project_name, language, build_system, None, false)
}

pub fn create_project_with_system_and_pm(
	project_name: &str,
	language: Language,
	build_system: BuildSystem,
	package_manager: PackageManager,
) -> Result<()> {
	let root = std::env::current_dir().context("Failed to get current directory")?;
	create_project_in(
		&root,
		project_name,
		language,
		build_system,
		Some(package_manager),
		false,
	)
}

fn create_project_in(
	root: &Path,
	project_name: &str,
	language: Language,
	build_system: BuildSystem,
	package_manager: Option<PackageManager>,
	validate_name: bool,
) -> Result<()> {
	if validate_name {
		validate_project_name(project_name)?;
	}
	if matches!(
		(package_manager, build_system),
		(Some(PackageManager::Conan), BuildSystem::Makefile)
	) {
		anyhow::bail!("Conan integration is supported only with CMake");
	}

	ensure_no_conflicting_integrations(root, build_system)?;
	let project_id = project_build_name(project_name);
	let hello_world_content = language.generate_helloworld_content();
	let generator = get_generator(build_system);
	let build_file_content = generator.generate_build_file(language, &project_id);

	let src_dir = root.join("src");
	let include_dir = root.join("include");
	let source_file = src_dir.join(format!("main.{}", language.extension()));
	let build_file = root.join(generator.extension());
	let vscode_available = command_succeeds(root, "code", &["--version"]);
	let mut generated_files = vec![
		source_file.clone(),
		build_file,
		root.join(".gitignore"),
		root.join(".editorconfig"),
		root.join(".clang-format"),
		root.join("README.md"),
		root.join(".gitattributes"),
	];
	if vscode_available {
		generated_files.extend([
			root.join(".vscode/settings.json"),
			root.join(".vscode/launch.json"),
			root.join(".vscode/tasks.json"),
		]);
	}

	if let Some(pm) = package_manager {
		let pm_generator = get_package_manager_generator(pm);
		generated_files.push(root.join(pm_generator.extension()));
	}

	for path in &generated_files {
		if path.exists() {
			anyhow::bail!(
				"Refusing to overwrite existing project file: {}",
				path.display()
			);
		}
	}

	for directory in [&src_dir, &include_dir] {
		if let Ok(metadata) = fs::symlink_metadata(directory)
			&& (metadata.file_type().is_symlink() || !metadata.is_dir())
		{
			anyhow::bail!(
				"Project directory must be a real directory: {}",
				directory.display()
			);
		}
	}

	if src_dir.exists() && directory_has_entries(&src_dir) {
		anyhow::bail!(
			"Refusing to initialize over existing sources in {}",
			src_dir.display()
		);
	}
	if include_dir.exists() && directory_has_entries(&include_dir) {
		anyhow::bail!(
			"Refusing to initialize over existing headers in {}",
			include_dir.display()
		);
	}

	fs::create_dir_all(&src_dir).context("Failed to create src directory")?;
	fs::create_dir_all(&include_dir).context("Failed to create include directory")?;
	write_new_file(&source_file, &hello_world_content)?;
	write_new_file(&root.join(generator.extension()), &build_file_content)?;
	write_new_file(
		&root.join(".gitignore"),
		&templates::generate_gitignore(language),
	)?;
	write_new_file(
		&root.join(".editorconfig"),
		&templates::generate_editorconfig(),
	)?;
	write_new_file(
		&root.join(".clang-format"),
		&templates::generate_clang_format_config(language),
	)?;

	if vscode_available {
		let vscode_dir = root.join(".vscode");
		fs::create_dir_all(&vscode_dir).context("Failed to create .vscode directory")?;
		write_new_file(
			&vscode_dir.join("settings.json"),
			&templates::generate_vscode_settings(language),
		)?;
		write_new_file(
			&vscode_dir.join("launch.json"),
			&templates::generate_vscode_launch_config(&project_id),
		)?;
		write_new_file(
			&vscode_dir.join("tasks.json"),
			&templates::generate_vscode_tasks_config_for(build_system),
		)?;
		println!("Generated VSCode configuration");
	}

	write_new_file(
		&root.join("README.md"),
		&templates::generate_readme(&project_id, language),
	)?;
	write_new_file(
		&root.join(".gitattributes"),
		templates::generate_gitattributes(),
	)?;

	if let Some(pm) = package_manager {
		let pm_generator = get_package_manager_generator(pm);
		write_new_file(
			&root.join(pm_generator.extension()),
			&pm_generator.generate_manifest(&project_id),
		)?;
		configure_package_manager_build(root, pm, build_system)?;
		println!("Generated {} configuration", pm);
	}

	if command_succeeds(root, "git", &["init", "-q"]) {
		println!("Initialized git repository");
	}

	println!(
		"Created {} project: {} with {}",
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
	new_project_with_system_and_pm(project_name, language, build_system, None)
}

pub fn new_project_with_system_and_pm(
	project_name: &str,
	language: Language,
	build_system: BuildSystem,
	package_manager: Option<PackageManager>,
) -> Result<()> {
	validate_project_name(project_name)?;
	let root = project_path(project_name)?;
	fs::create_dir(&root)
		.with_context(|| format!("Failed to create directory '{}'", project_name))?;
	let result = create_project_in(
		&root,
		project_name,
		language,
		build_system,
		package_manager,
		true,
	);
	if result.is_err() {
		let _ = fs::remove_dir_all(&root);
	}
	result
}

pub fn init_project(language: Language) -> Result<()> {
	init_project_with_system(language, BuildSystem::Makefile)
}

pub fn init_project_with_system(language: Language, build_system: BuildSystem) -> Result<()> {
	init_project_with_system_and_pm_internal(language, build_system, None)
}

pub fn init_project_with_system_and_pm(
	language: Language,
	build_system: BuildSystem,
	package_manager: PackageManager,
) -> Result<()> {
	init_project_with_system_and_pm_internal(language, build_system, Some(package_manager))
}

fn init_project_with_system_and_pm_internal(
	language: Language,
	build_system: BuildSystem,
	package_manager: Option<PackageManager>,
) -> Result<()> {
	let current_dir = std::env::current_dir().context("Failed to get current directory")?;
	let current_dir_name = current_dir
		.file_name()
		.and_then(|name| name.to_str())
		.unwrap_or("project")
		.to_string();
	create_project_in(
		&current_dir,
		&current_dir_name,
		language,
		build_system,
		package_manager,
		false,
	)?;
	println!("Initialized {} project in current directory", language);
	Ok(())
}

pub(crate) fn configure_package_manager_build(
	root: &Path,
	package_manager: PackageManager,
	build_system: BuildSystem,
) -> Result<()> {
	match (package_manager, build_system) {
		(PackageManager::Vcpkg, BuildSystem::CMake) => {
			let path = root.join("CMakeLists.txt");
			let mut content = fs::read_to_string(&path).context("Failed to read CMakeLists.txt")?;
			if !content.contains("vcpkg/scripts/buildsystems/vcpkg.cmake") {
				let insertion = content
					.find('\n')
					.map(|index| index + 1)
					.unwrap_or(content.len());
				let block = "if(DEFINED ENV{VCPKG_ROOT} AND EXISTS \"$ENV{VCPKG_ROOT}/scripts/buildsystems/vcpkg.cmake\")\n    set(CMAKE_TOOLCHAIN_FILE \"$ENV{VCPKG_ROOT}/scripts/buildsystems/vcpkg.cmake\" CACHE STRING \"\" FORCE)\nelseif(EXISTS \"${CMAKE_SOURCE_DIR}/vcpkg/scripts/buildsystems/vcpkg.cmake\")\n    set(CMAKE_TOOLCHAIN_FILE \"${CMAKE_SOURCE_DIR}/vcpkg/scripts/buildsystems/vcpkg.cmake\" CACHE STRING \"\" FORCE)\nendif()\n";
				content.insert_str(insertion, block);
				file_handler::write_atomic(&path, &content)?;
			}
		}
		(PackageManager::Vcpkg, BuildSystem::Makefile) => {
			let path = ["Makefile", "makefile", "GNUmakefile"]
				.iter()
				.map(|name| root.join(name))
				.find(|path| path.is_file())
				.ok_or_else(|| anyhow::anyhow!("Makefile not found"))?;
			let mut content = fs::read_to_string(&path).context("Failed to read Makefile")?;
			if !content.contains("VCPKG_INCLUDEDIR") && !content.contains("VCPKG_LIBDIR") {
				let block = "VCPKG_ROOT ?= vcpkg\nVCPKG_TRIPLET ?= $(shell arch=\"$$(uname -m)\"; if test \"$$arch\" = x86_64; then printf x64-linux; elif test \"$$arch\" = aarch64; then printf arm64-linux; elif test \"$$arch\" = armv7l; then printf arm-linux; else printf \"$$arch-linux\"; fi)\nVCPKG_INCLUDEDIR ?= $(VCPKG_ROOT)/installed/$(VCPKG_TRIPLET)/include\nVCPKG_LIBDIR ?= $(VCPKG_ROOT)/installed/$(VCPKG_TRIPLET)/lib\nCPPFLAGS += -I$(VCPKG_INCLUDEDIR)\nLDFLAGS += -L$(VCPKG_LIBDIR)\n\n";
				content.push_str(block);
				file_handler::write_atomic(&path, &content)?;
			}
		}
		(PackageManager::Conan, BuildSystem::CMake) => {
			let path = root.join("CMakePresets.json");
			if !path.exists() {
				let preset = "{\"version\":3,\"configurePresets\":[{\"name\":\"conan\",\"binaryDir\":\"${sourceDir}/build\",\"cacheVariables\":{\"CMAKE_TOOLCHAIN_FILE\":\"${sourceDir}/build/conan_toolchain.cmake\"}}]}\n";
				file_handler::write_atomic(&path, preset)?;
			}
		}
		(PackageManager::Conan, BuildSystem::Makefile) => {
			anyhow::bail!("Conan integration is supported only with CMake")
		}
	}
	Ok(())
}

fn ensure_no_conflicting_integrations(root: &Path, build_system: BuildSystem) -> Result<()> {
	let selected_build_file = match build_system {
		BuildSystem::CMake => "CMakeLists.txt",
		BuildSystem::Makefile => "Makefile",
	};
	for name in ["CMakeLists.txt", "Makefile", "makefile", "GNUmakefile"] {
		if name != selected_build_file && root.join(name).is_file() {
			anyhow::bail!("A different build system already exists: {}", name);
		}
	}
	let existing_package_manager =
		if root.join("conanfile.txt").is_file() || root.join("conanfile.py").is_file() {
			Some(PackageManager::Conan)
		} else if root.join("vcpkg.json").is_file() {
			Some(PackageManager::Vcpkg)
		} else {
			None
		};
	if existing_package_manager.is_some() {
		anyhow::bail!("A package manager configuration already exists");
	}
	Ok(())
}

fn directory_has_entries(path: &Path) -> bool {
	fs::read_dir(path)
		.map(|mut entries| entries.next().is_some())
		.unwrap_or(false)
}

fn command_succeeds(root: &Path, command: &str, args: &[&str]) -> bool {
	Command::new(command)
		.args(args)
		.current_dir(root)
		.output()
		.map(|output| output.status.success())
		.unwrap_or(false)
}

pub(crate) fn project_build_name(name: &str) -> String {
	let identifier = project_identifier(name);
	if file_handler::validate_project_name(name).is_ok() && !identifier.starts_with('_') {
		name.to_string()
	} else {
		identifier
	}
}

pub(crate) fn project_identifier(name: &str) -> String {
	let mut identifier = String::new();
	for character in name.chars() {
		if character.is_ascii_alphanumeric() || character == '_' {
			identifier.push(character);
		} else if character == '-' {
			identifier.push('_');
		}
	}
	if identifier.is_empty() {
		return "project".to_string();
	}
	if identifier
		.chars()
		.next()
		.is_some_and(|character| character.is_ascii_digit())
	{
		identifier.insert(0, '_');
	}
	identifier
}

pub fn current_project_path() -> Result<PathBuf> {
	std::env::current_dir().context("Failed to get current directory")
}
