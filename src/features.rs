use anyhow::{Context, Result, bail};
use std::fs;
use std::path::{Path, PathBuf};

use crate::{BuildSystem, PackageManager, file_handler};

pub fn detect_build_file() -> Result<Option<PathBuf>> {
	if Path::new("CMakeLists.txt").is_file() {
		return Ok(Some(PathBuf::from("CMakeLists.txt")));
	}
	for name in ["Makefile", "makefile", "GNUmakefile"] {
		if Path::new(name).is_file() {
			return Ok(Some(PathBuf::from(name)));
		}
	}
	Ok(None)
}

pub fn detect_build_system() -> Result<Option<BuildSystem>> {
	Ok(detect_build_file()?.map(|path| {
		if path.extension().is_some_and(|extension| extension == "txt") {
			BuildSystem::CMake
		} else {
			BuildSystem::Makefile
		}
	}))
}

pub fn detect_package_manager() -> Result<Option<PackageManager>> {
	let conan = Path::new("conanfile.txt").is_file() || Path::new("conanfile.py").is_file();
	let vcpkg = Path::new("vcpkg.json").is_file();
	match (conan, vcpkg) {
		(true, true) => bail!("Both Conan and vcpkg are configured; choose one package manager"),
		(true, false) => Ok(Some(PackageManager::Conan)),
		(false, true) => Ok(Some(PackageManager::Vcpkg)),
		(false, false) => Ok(None),
	}
}

pub fn convert_build_system(from: BuildSystem, to: BuildSystem, project_name: &str) -> Result<()> {
	convert_build_system_with_prompt(from, to, project_name, false)
}

pub fn convert_build_system_interactive(
	from: BuildSystem,
	to: BuildSystem,
	project_name: &str,
) -> Result<()> {
	convert_build_system_with_prompt(from, to, project_name, true)
}

fn convert_build_system_with_prompt(
	from: BuildSystem,
	to: BuildSystem,
	project_name: &str,
	interactive: bool,
) -> Result<()> {
	if from == to {
		bail!("Project already uses {}. No conversion needed.", to);
	}
	if detect_package_manager()? == Some(PackageManager::Conan) && to == BuildSystem::Makefile {
		bail!("Conan integration is supported only with CMake");
	}

	let project_id = crate::project_build_name(project_name);
	let new_generator = crate::get_generator(to);
	let old_build_file = detect_build_file()?
		.ok_or_else(|| anyhow::anyhow!("No {} build file found in current directory", from))?;
	let detected_is_cmake = old_build_file
		.file_name()
		.is_some_and(|name| name == "CMakeLists.txt");
	if detected_is_cmake != (from == BuildSystem::CMake) {
		bail!("The detected build file does not match {}", from);
	}
	let new_build_file = PathBuf::from(new_generator.extension());

	let old_build_file_exists = match fs::symlink_metadata(&old_build_file) {
		Ok(metadata) => {
			if metadata.is_dir() {
				bail!(
					"Old build system path '{}' is not a file",
					old_build_file.display()
				);
			}
			true
		}
		Err(error) if error.kind() == std::io::ErrorKind::NotFound => false,
		Err(error) => {
			return Err(error)
				.with_context(|| format!("Failed to inspect '{}'", old_build_file.display()));
		}
	};

	file_handler::ensure_path_available(&new_build_file)
		.with_context(|| format!("Cannot create '{}'", new_build_file.display()))?;

	let language = crate::languages::Language::from_project_structure_with_prompt(interactive)?;
	let build_file_content = new_generator.generate_build_file(language, &project_id);
	file_handler::atomic_write_new(&new_build_file, &build_file_content).with_context(|| {
		format!(
			"Failed to write new build system file '{}'",
			new_build_file.display()
		)
	})?;
	if let Some(package_manager) = detect_package_manager()? {
		crate::configure_package_manager_build(Path::new("."), package_manager, to)?;
	}

	if old_build_file_exists {
		let backup = old_build_file.with_file_name(format!(
			"{}.sticks.bak",
			old_build_file
				.file_name()
				.and_then(|name| name.to_str())
				.unwrap_or("build")
		));
		file_handler::ensure_path_available(&backup)
			.with_context(|| format!("Cannot create backup '{}'", backup.display()))?;
		fs::rename(&old_build_file, &backup).with_context(|| {
			format!(
				"Failed to preserve old build file '{}'",
				old_build_file.display()
			)
		})?;
		println!("✓ Preserved old build file as {}", backup.display());
	}

	println!("✓ Successfully converted project from {} to {}", from, to);
	Ok(())
}

pub fn add_package_manager_to_project(pm: PackageManager, project_name: &str) -> Result<()> {
	let project_id = crate::project_build_name(project_name);

	match detect_package_manager()? {
		Some(existing) if existing == pm => {
			bail!("Project already uses {}. No changes needed.", pm);
		}
		Some(existing) => {
			bail!(
				"Project already uses {}. Remove it before adding {}.",
				existing,
				pm
			);
		}
		None => {}
	}

	if pm == PackageManager::Conan && matches!(detect_build_system()?, Some(BuildSystem::Makefile))
	{
		bail!("Conan integration is supported only with CMake");
	}

	let pm_generator = crate::get_package_manager_generator(pm);
	let manifest_path = Path::new(pm_generator.extension());
	file_handler::ensure_path_available(manifest_path)
		.with_context(|| format!("Cannot create '{}'", manifest_path.display()))?;
	let manifest = pm_generator.generate_manifest(&project_id);
	file_handler::atomic_write_new(manifest_path, &manifest)
		.with_context(|| format!("Failed to write {} manifest", pm_generator.name()))?;
	if let Some(build_system) = detect_build_system()? {
		crate::configure_package_manager_build(Path::new("."), pm, build_system)?;
	}

	println!("✓ Generated {} configuration", pm);
	println!(
		"📝 Next steps: {}",
		pm_generator.generate_install_instructions()
	);

	Ok(())
}

pub fn remove_package_manager_from_project(pm: PackageManager) -> Result<()> {
	let pm_generator = crate::get_package_manager_generator(pm);

	if !Path::new(pm_generator.extension()).exists() {
		bail!(
			"{} not found in project. Nothing to remove.",
			pm_generator.name()
		);
	}

	if pm == PackageManager::Conan {
		let preset_path = Path::new("CMakePresets.json");
		if preset_path.is_file() {
			let content =
				fs::read_to_string(preset_path).context("Failed to read CMakePresets.json")?;
			if content.contains("conan_toolchain.cmake") {
				fs::remove_file(preset_path).context("Failed to remove Conan CMake preset")?;
			}
		}
	}
	if pm == PackageManager::Vcpkg {
		let cmake_path = Path::new("CMakeLists.txt");
		if cmake_path.is_file() {
			let content =
				fs::read_to_string(cmake_path).context("Failed to read CMakeLists.txt")?;
			if let Some(updated) = remove_vcpkg_toolchain_block(&content) {
				file_handler::write_atomic(cmake_path, &updated)?;
			}
		}
		if let Some(makefile_path) = ["Makefile", "makefile", "GNUmakefile"]
			.iter()
			.map(std::path::PathBuf::from)
			.find(|path| path.is_file())
		{
			let content = fs::read_to_string(&makefile_path).context("Failed to read Makefile")?;
			let updated = remove_vcpkg_makefile_integration(&content);
			if updated != content {
				file_handler::write_atomic(&makefile_path, &updated)?;
			}
		}
	}

	fs::remove_file(pm_generator.extension())
		.with_context(|| format!("Failed to remove {} file", pm_generator.name()))?;

	println!("✓ Removed {} configuration", pm);
	Ok(())
}

fn remove_vcpkg_toolchain_block(content: &str) -> Option<String> {
	let start = content
		.find("if(DEFINED ENV{VCPKG_ROOT}")
		.or_else(|| content.find("if(EXISTS \"${CMAKE_SOURCE_DIR}/vcpkg/"))?;
	let end = content[start..]
		.find("endif()")
		.map(|offset| start + offset + "endif()".len())?;
	let mut updated = content.to_string();
	updated.drain(start..end);
	Some(updated)
}

fn remove_vcpkg_makefile_integration(content: &str) -> String {
	content
		.lines()
		.filter(|line| {
			let trimmed = line.trim();
			!trimmed.starts_with("VCPKG_ROOT")
				&& !trimmed.starts_with("VCPKG_TRIPLET")
				&& !trimmed.starts_with("VCPKG_INCLUDEDIR")
				&& !trimmed.starts_with("VCPKG_LIBDIR")
				&& trimmed != "CPPFLAGS += -I$(VCPKG_INCLUDEDIR)"
				&& trimmed != "LDFLAGS += -L$(VCPKG_LIBDIR)"
		})
		.collect::<Vec<_>>()
		.join("\n")
		+ "\n"
}

pub fn list_features() -> Result<()> {
	println!("\n📦 Project Features:");
	println!("====================\n");

	match detect_build_system()? {
		Some(bs) => println!("  Build System:     {}", bs),
		None => println!("  Build System:     (none detected)"),
	}

	match detect_package_manager()? {
		Some(pm) => println!("  Package Manager:  {}", pm),
		None => println!("  Package Manager:  (none configured)"),
	}

	let has_src = Path::new("src").exists();
	let has_include = Path::new("include").exists();
	let has_vscode = Path::new(".vscode").exists();
	let has_gitignore = Path::new(".gitignore").exists();
	let has_clang_format = Path::new(".clang-format").exists();

	println!("  Src directory:    {}", if has_src { "✓" } else { "✗" });
	println!(
		"  Include directory: {}",
		if has_include { "✓" } else { "✗" }
	);
	println!("  VSCode config:    {}", if has_vscode { "✓" } else { "✗" });
	println!(
		"  .gitignore:       {}",
		if has_gitignore { "✓" } else { "✗" }
	);
	println!(
		"  .clang-format:    {}",
		if has_clang_format { "✓" } else { "✗" }
	);

	println!("\n");
	Ok(())
}
