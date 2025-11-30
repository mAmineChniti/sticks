use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

use crate::{BuildSystem, PackageManager};

/// Detect the current build system in a project
pub fn detect_build_system() -> Result<Option<BuildSystem>> {
	if Path::new("CMakeLists.txt").exists() {
		Ok(Some(BuildSystem::CMake))
	} else if Path::new("Makefile").exists() {
		Ok(Some(BuildSystem::Makefile))
	} else {
		Ok(None)
	}
}

/// Detect if a package manager is already configured
pub fn detect_package_manager() -> Result<Option<PackageManager>> {
	if Path::new("conanfile.txt").exists() || Path::new("conanfile.py").exists() {
		Ok(Some(PackageManager::Conan))
	} else if Path::new("vcpkg.json").exists() {
		Ok(Some(PackageManager::Vcpkg))
	} else {
		Ok(None)
	}
}

/// Convert from one build system to another
pub fn convert_build_system(from: BuildSystem, to: BuildSystem, project_name: &str) -> Result<()> {
	if from == to {
		anyhow::bail!("Project already uses {}. No conversion needed.", to);
	}

	let language = crate::languages::Language::from_project_structure()?;

	// Remove old build system file
	match from {
		BuildSystem::Makefile => {
			if Path::new("Makefile").exists() {
				fs::remove_file("Makefile").context("Failed to remove old Makefile")?;
				println!("✓ Removed Makefile");
			}
		}
		BuildSystem::CMake => {
			if Path::new("CMakeLists.txt").exists() {
				fs::remove_file("CMakeLists.txt").context("Failed to remove old CMakeLists.txt")?;
				println!("✓ Removed CMakeLists.txt");
			}
		}
	}

	// Generate new build system file
	let generator = crate::get_generator(to);
	let build_file_content = generator.generate_build_file(language, project_name);
	fs::write(generator.extension(), build_file_content)
		.context("Failed to write new build system file")?;

	println!("✓ Successfully converted project from {} to {}", from, to);
	Ok(())
}

/// Add a package manager to an existing project
pub fn add_package_manager_to_project(pm: PackageManager, project_name: &str) -> Result<()> {
	// Check if package manager already exists
	if let Ok(Some(existing)) = detect_package_manager() {
		if existing == pm {
			anyhow::bail!("Project already uses {}. No changes needed.", pm);
		} else {
			println!(
				"⚠️  Warning: Project already has {} configured. Adding {} as well.",
				existing, pm
			);
		}
	}

	let pm_generator = crate::get_package_manager_generator(pm);
	let manifest = pm_generator.generate_manifest(project_name);
	fs::write(pm_generator.extension(), manifest)
		.with_context(|| format!("Failed to write {} manifest", pm_generator.name()))?;

	println!("✓ Generated {} configuration", pm);
	println!(
		"📝 Next steps: {}",
		pm_generator.generate_install_instructions()
	);

	Ok(())
}

/// Remove a package manager from a project
pub fn remove_package_manager_from_project(pm: PackageManager) -> Result<()> {
	let pm_generator = crate::get_package_manager_generator(pm);

	if !Path::new(pm_generator.extension()).exists() {
		anyhow::bail!(
			"{} not found in project. Nothing to remove.",
			pm_generator.name()
		);
	}

	fs::remove_file(pm_generator.extension())
		.with_context(|| format!("Failed to remove {} file", pm_generator.name()))?;

	println!("✓ Removed {} configuration", pm);
	Ok(())
}

/// List all features detected in the current project
pub fn list_features() -> Result<()> {
	println!("\n📦 Project Features:");
	println!("====================\n");

	// Build system
	match detect_build_system()? {
		Some(bs) => println!("  Build System:     {}", bs),
		None => println!("  Build System:     (none detected)"),
	}

	// Package manager
	match detect_package_manager()? {
		Some(pm) => println!("  Package Manager:  {}", pm),
		None => println!("  Package Manager:  (none configured)"),
	}

	// Project structure
	let has_src = Path::new("src").exists();
	let has_vscode = Path::new(".vscode").exists();
	let has_gitignore = Path::new(".gitignore").exists();
	let has_clang_format = Path::new(".clang-format").exists();

	println!("  Src directory:    {}", if has_src { "✓" } else { "✗" });
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
