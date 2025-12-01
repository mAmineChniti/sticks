use sticks::{get_package_manager_generator, PackageManager};

#[test]
fn test_package_manager_display() {
	assert_eq!(format!("{}", PackageManager::Conan), "Conan");
	assert_eq!(format!("{}", PackageManager::Vcpkg), "vcpkg");
}

#[test]
fn test_package_manager_from_str() {
	assert!(matches!(
		"conan".parse::<PackageManager>(),
		Ok(PackageManager::Conan)
	));
	assert!(matches!(
		"CONAN".parse::<PackageManager>(),
		Ok(PackageManager::Conan)
	));
	assert!(matches!(
		"vcpkg".parse::<PackageManager>(),
		Ok(PackageManager::Vcpkg)
	));
	assert!(matches!(
		"VCPKG".parse::<PackageManager>(),
		Ok(PackageManager::Vcpkg)
	));
	assert!("npm".parse::<PackageManager>().is_err());
	assert!("pip".parse::<PackageManager>().is_err());
}

#[test]
fn test_package_manager_equality() {
	assert_eq!(PackageManager::Conan, PackageManager::Conan);
	assert_eq!(PackageManager::Vcpkg, PackageManager::Vcpkg);
	assert_ne!(PackageManager::Conan, PackageManager::Vcpkg);
}

/// Verifies the Conan package manager generator produces the expected name, file extension, and manifest sections.
///
/// Asserts that the generator for `PackageManager::Conan` reports the name "Conan" and extension "conanfile.txt", and that a manifest generated for "test_project" contains the sections `[requires]`, `[generators]`, `CMakeDeps`, `CMakeToolchain`, and `[options]`.
///
/// # Examples
///
/// ```
/// let generator = get_package_manager_generator(PackageManager::Conan);
/// assert_eq!(generator.name(), "Conan");
/// assert_eq!(generator.extension(), "conanfile.txt");
///
/// let manifest = generator.generate_manifest("test_project");
/// assert!(manifest.contains("[requires]"));
/// assert!(manifest.contains("[generators]"));
/// assert!(manifest.contains("CMakeDeps"));
/// assert!(manifest.contains("CMakeToolchain"));
/// assert!(manifest.contains("[options]"));
/// ```
#[test]
fn test_conan_generator() {
	let generator = get_package_manager_generator(PackageManager::Conan);
	assert_eq!(generator.name(), "Conan");
	assert_eq!(generator.extension(), "conanfile.txt");

	let manifest = generator.generate_manifest("test_project");
	assert!(manifest.contains("[requires]"));
	assert!(manifest.contains("[generators]"));
	assert!(manifest.contains("CMakeDeps"));
	assert!(manifest.contains("CMakeToolchain"));
	assert!(manifest.contains("[options]"));
}

#[test]
fn test_conan_install_instructions() {
	let generator = get_package_manager_generator(PackageManager::Conan);
	let instructions = generator.generate_install_instructions();
	assert!(instructions.contains("pip install conan"));
	assert!(instructions.contains("conanfile.txt"));
	assert!(instructions.contains("conan install"));
}

#[test]
fn test_vcpkg_generator() {
	let generator = get_package_manager_generator(PackageManager::Vcpkg);
	assert_eq!(generator.name(), "vcpkg");
	assert_eq!(generator.extension(), "vcpkg.json");

	let manifest = generator.generate_manifest("my_project");
	assert!(manifest.contains("\"name\": \"my_project\""));
	assert!(manifest.contains("\"version\": \"0.1.0\""));
	assert!(manifest.contains("\"dependencies\""));
}

#[test]
fn test_vcpkg_install_instructions() {
	let generator = get_package_manager_generator(PackageManager::Vcpkg);
	let instructions = generator.generate_install_instructions();
	assert!(instructions.contains("git clone https://github.com/Microsoft/vcpkg.git"));
	assert!(instructions.contains("bootstrap-vcpkg.sh"));
	assert!(instructions.contains("vcpkg.json"));
	assert!(instructions.contains("CMAKE_TOOLCHAIN_FILE"));
}

#[test]
fn test_get_package_manager_generator_conan() {
	let generator = get_package_manager_generator(PackageManager::Conan);
	assert_eq!(generator.name(), "Conan");
	assert_eq!(generator.extension(), "conanfile.txt");
}

#[test]
fn test_get_package_manager_generator_vcpkg() {
	let generator = get_package_manager_generator(PackageManager::Vcpkg);
	assert_eq!(generator.name(), "vcpkg");
	assert_eq!(generator.extension(), "vcpkg.json");
}

#[test]
fn test_conan_manifest_content_completeness() {
	let generator = get_package_manager_generator(PackageManager::Conan);
	let manifest = generator.generate_manifest("example");
	assert!(manifest.contains("[requires]"));
	assert!(manifest.contains("[generators]"));
	assert!(manifest.contains("[options]"));
	assert!(manifest.contains("[imports]"));
}

#[test]
fn test_vcpkg_manifest_structure() {
	let generator = get_package_manager_generator(PackageManager::Vcpkg);
	let manifest = generator.generate_manifest("test_app");
	// Verify basic JSON structure
	assert!(manifest.contains("{"));
	assert!(manifest.contains("}"));
	assert!(manifest.contains("\"name\""));
	assert!(manifest.contains("test_app"));
}