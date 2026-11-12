use anyhow::{Context, Result, bail};
use std::fs;
use std::path::{Path, PathBuf};

use crate::{BuildSystem, PackageManager, file_handler};

pub fn detect_build_file() -> Result<Option<PathBuf>> {
	let current = std::env::current_dir().context("Failed to get current directory")?;
	detect_build_file_in(&current)
}

/// CWD-independent variant: inspects `dir` instead of the process working
/// directory. Prefer this from library code and tests to avoid `serial`
/// bottlenecks and global-state races.
pub fn detect_build_file_in(dir: &Path) -> Result<Option<PathBuf>> {
	if dir.join("CMakeLists.txt").is_file() {
		return Ok(Some(dir.join("CMakeLists.txt")));
	}
	for name in ["Makefile", "makefile", "GNUmakefile"] {
		if dir.join(name).is_file() {
			return Ok(Some(dir.join(name)));
		}
	}
	Ok(None)
}

pub fn detect_build_system() -> Result<Option<BuildSystem>> {
	let current = std::env::current_dir().context("Failed to get current directory")?;
	detect_build_system_in(&current)
}

/// CWD-independent variant of [`detect_build_system`].
pub fn detect_build_system_in(dir: &Path) -> Result<Option<BuildSystem>> {
	Ok(detect_build_file_in(dir)?.map(|path| {
		if path
			.file_name()
			.is_some_and(|name| name == "CMakeLists.txt")
		{
			BuildSystem::CMake
		} else {
			BuildSystem::Makefile
		}
	}))
}

pub fn detect_package_manager() -> Result<Option<PackageManager>> {
	let current = std::env::current_dir().context("Failed to get current directory")?;
	detect_package_manager_in(&current)
}

/// CWD-independent variant of [`detect_package_manager`].
pub fn detect_package_manager_in(dir: &Path) -> Result<Option<PackageManager>> {
	let conan = dir.join("conanfile.txt").is_file() || dir.join("conanfile.py").is_file();
	let vcpkg = dir.join("vcpkg.json").is_file();
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
	if to == BuildSystem::CMake {
		crate::project_ops::ensure_cmake_presets(Path::new("."))?;
	}

	if old_build_file_exists {
		let base_name = old_build_file
			.file_name()
			.and_then(|name| name.to_str())
			.unwrap_or("build");
		// Find a free backup name so repeated conversions don't fail when
		// `<file>.sticks.bak` already exists.
		let mut backup = old_build_file.with_file_name(format!("{}.sticks.bak", base_name));
		let mut counter = 1;
		while backup.exists() {
			counter += 1;
			backup = old_build_file.with_file_name(format!("{}.sticks.bak.{}", base_name, counter));
		}
		if file_handler::dry_run() {
			println!(
				"would preserve old build file {} as {}",
				old_build_file.display(),
				backup.display()
			);
		} else {
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

	// Compute the preset update up front so nothing is written before the
	// manifest removal (the only genuinely destructive step) succeeds.
	let mut preset_update: Option<Option<String>> = None;
	if pm == PackageManager::Conan {
		let preset_path = Path::new("CMakePresets.json");
		if preset_path.is_file() {
			let content =
				fs::read_to_string(preset_path).context("Failed to read CMakePresets.json")?;
			if content.contains("conan_toolchain.cmake") {
				// `None` = delete the file (it held nothing but Conan presets).
				preset_update = Some(strip_conan_presets(&content)?);
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

	file_handler::remove_with_backup(Path::new(pm_generator.extension()))?;

	if let Some(Some(updated)) = preset_update {
		file_handler::write_atomic(Path::new("CMakePresets.json"), &updated)
			.context("Failed to update CMakePresets.json")?;
	} else if preset_update.is_some() {
		// The file contained only Conan presets, so it is safe to remove.
		let _ = file_handler::remove_with_backup(Path::new("CMakePresets.json"));
	}

	println!("✓ Removed {} configuration", pm);
	Ok(())
}

/// Removes only the Conan presets from a `CMakePresets.json`, leaving every
/// other preset the user defined intact.
///
/// Returns `Ok(None)` when nothing but Conan presets remained (the caller then
/// deletes the file), and `Ok(Some(json))` with the rewritten file otherwise.
fn strip_conan_presets(content: &str) -> Result<Option<String>> {
	const PRESET_ARRAYS: [&str; 3] = ["configurePresets", "buildPresets", "testPresets"];

	let mut root: serde_json::Value =
		serde_json::from_str(content).context("Failed to parse CMakePresets.json as JSON")?;
	let object = root
		.as_object_mut()
		.context("CMakePresets.json root must be an object")?;

	let mut removed_any = false;
	for array_name in PRESET_ARRAYS {
		let Some(presets) = object
			.get_mut(array_name)
			.and_then(|value| value.as_array_mut())
		else {
			continue;
		};
		let before = presets.len();
		presets.retain(|preset| !preset.to_string().contains("conan_toolchain.cmake"));
		if presets.len() != before {
			removed_any = true;
		}
	}
	if !removed_any {
		return Ok(Some(content.to_string()));
	}

	// Drop preset arrays that are now empty.
	for name in PRESET_ARRAYS {
		if object
			.get(name)
			.is_some_and(|value| value.as_array().is_some_and(Vec::is_empty))
		{
			object.remove(name);
		}
	}
	if !PRESET_ARRAYS
		.iter()
		.any(|name| object.get(*name).is_some_and(|value| !value.is_null()))
	{
		// Nothing but Conan presets remained; the whole file goes away.
		return Ok(None);
	}
	// Top-level metadata only makes sense while presets are present.
	if !PRESET_ARRAYS
		.iter()
		.all(|name| object.get(*name).is_some_and(|value| !value.is_null()))
	{
		for name in ["version", "cmakeMinimumRequired", "vendor"] {
			object.remove(name);
		}
	}

	serde_json::to_string_pretty(&root)
		.map(Some)
		.context("Failed to serialize CMakePresets.json")
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

/// Files and directories sticks generates that are not required to build
/// the project. `sticks eject` moves each to a `.sticks.bak` backup so the
/// project keeps working while remaining recoverable.
const EJECT_CANDIDATES: &[&str] = &[
	".vscode",
	".clang-format",
	".editorconfig",
	".gitattributes",
	"CMakePresets.json",
];

/// Remove sticks-specific metadata, leaving a plain working project.
/// Build files (Makefile/CMakeLists.txt), package-manager manifests
/// (conanfile.*, vcpkg.json) and sources are standard files and are kept.
pub fn eject_project() -> Result<()> {
	eject_project_in(Path::new("."))
}

/// CWD-independent variant of [`eject_project`].
pub fn eject_project_in(dir: &Path) -> Result<()> {
	let mut removed = 0usize;
	for name in EJECT_CANDIDATES {
		let path = dir.join(name);
		if !path.exists() {
			continue;
		}
		file_handler::remove_with_backup(&path)?;
		removed += 1;
	}
	// Leftover `.sticks.bak` files are already backups; report them but do
	// not delete user data silently.
	let stray_backups = fs::read_dir(dir)
		.map(|entries| {
			entries
				.filter_map(|entry| entry.ok())
				.filter(|entry| {
					entry
						.file_name()
						.to_string_lossy()
						.contains(file_handler::BACKUP_SUFFIX)
				})
				.count()
		})
		.unwrap_or(0);
	if stray_backups > 0 {
		println!(
			"ℹ {} backup file(s) matching '*.sticks.bak' left in place; delete them when ready.",
			stray_backups
		);
	}
	println!("✓ Ejected sticks metadata ({removed} item(s) moved to backups)");
	println!("  Build files and package-manager manifests were left untouched.");
	Ok(())
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

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn strip_conan_presets_keeps_user_presets() {
		let content = r#"{
			"version": 3,
			"configurePresets": [
				{"name": "conan", "cacheVariables": {"CMAKE_TOOLCHAIN_FILE": "${sourceDir}/build/conan_toolchain.cmake"}},
				{"name": "ninja-clang", "generator": "Ninja"}
			],
			"buildPresets": [{"name": "release", "configurePreset": "ninja-clang"}]
		}"#;
		let updated = strip_conan_presets(content)
			.unwrap()
			.expect("file must survive: it holds non-Conan presets");
		assert!(!updated.contains("conan_toolchain.cmake"));
		assert!(updated.contains("ninja-clang"));
		assert!(updated.contains("release"));
	}

	#[test]
	fn strip_conan_presets_deletes_file_when_only_conan_remains() {
		let content = r#"{
			"version": 3,
			"configurePresets": [
				{"name": "conan", "cacheVariables": {"CMAKE_TOOLCHAIN_FILE": "${sourceDir}/build/conan_toolchain.cmake"}}
			]
		}"#;
		assert!(strip_conan_presets(content).unwrap().is_none());
	}

	#[test]
	fn strip_conan_presets_leaves_non_conan_file_untouched() {
		let content = r#"{"version": 3, "configurePresets": [{"name": "ninja"}]}"#;
		let updated = strip_conan_presets(content).unwrap().unwrap();
		assert!(updated.contains("ninja"));
	}

	#[test]
	fn strip_conan_presets_rejects_invalid_json() {
		assert!(strip_conan_presets("not json at all").is_err());
	}
}
