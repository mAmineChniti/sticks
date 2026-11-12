use anyhow::Result;

/// Add dependencies to the project
///
/// This function is now a wrapper around the universal DependencyManager
/// which automatically detects the build system and routes accordingly.
///
/// # Arguments
/// * `dependency_names` - List of dependencies to add
///
/// # Behavior
/// - If Conan or vcpkg is detected, routes to those handlers
/// - Falls back to Makefile or CMake based on detection
/// - Returns error if no build system is found
pub fn add_dependencies(dependency_names: &[String]) -> Result<()> {
	add_dependencies_with_options(dependency_names, false)
}

/// `system = true` manages OS packages (Makefile `install-deps` /
/// CMake `find_package`) instead of the project's package-manager manifest.
pub fn add_dependencies_with_options(dependency_names: &[String], system: bool) -> Result<()> {
	let _added =
		crate::dependency_manager::DependencyManager::add_with_options(dependency_names, system)?;
	Ok(())
}

/// Remove dependencies from the project
///
/// This function is now a wrapper around the universal DependencyManager
/// which automatically detects the build system and routes accordingly.
///
/// # Arguments
/// * `dependency_names` - List of dependencies to remove
///
/// # Behavior
/// - If Conan or vcpkg is detected, routes to those handlers
/// - Falls back to Makefile or CMake based on detection
/// - Returns error if dependency is not found
pub fn remove_dependencies(dependency_names: &[String]) -> Result<()> {
	remove_dependencies_with_options(dependency_names, false)
}

/// `system = true` mirrors [`add_dependencies_with_options`].
pub fn remove_dependencies_with_options(dependency_names: &[String], system: bool) -> Result<()> {
	let _removed = crate::dependency_manager::DependencyManager::remove_with_options(
		dependency_names,
		system,
	)?;
	Ok(())
}

/// List all dependencies in the current project
pub fn list_dependencies() -> Result<()> {
	crate::dependency_manager::DependencyManager::list()
}
