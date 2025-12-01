use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PackageManager {
	Conan,
	Vcpkg,
}

impl std::fmt::Display for PackageManager {
	/// Formats the PackageManager as its human-readable name.
	///
	/// # Examples
	///
	/// ```
	/// use crate::package_managers::PackageManager;
	///
	/// assert_eq!(format!("{}", PackageManager::Conan), "Conan");
	/// assert_eq!(format!("{}", PackageManager::Vcpkg), "vcpkg");
	/// ```
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			PackageManager::Conan => write!(f, "Conan"),
			PackageManager::Vcpkg => write!(f, "vcpkg"),
		}
	}
}

impl FromStr for PackageManager {
	type Err = anyhow::Error;

	/// Parses a case-insensitive package manager name into a `PackageManager`.
	///
	/// Accepts `"conan"` or `"vcpkg"` (case-insensitive) and returns the corresponding variant; returns an error for any other input.
	///
	/// # Examples
	///
	/// ```
	/// use std::str::FromStr;
	/// use crate::package_managers::PackageManager;
	///
	/// assert_eq!(PackageManager::from_str("Conan").unwrap(), PackageManager::Conan);
	/// assert_eq!(PackageManager::from_str("vcpkg").unwrap(), PackageManager::Vcpkg);
	/// assert!(PackageManager::from_str("unknown").is_err());
	/// ```
	fn from_str(input: &str) -> Result<PackageManager, Self::Err> {
		match input.to_lowercase().as_str() {
			"conan" => Ok(PackageManager::Conan),
			"vcpkg" => Ok(PackageManager::Vcpkg),
			_ => anyhow::bail!(
				"Unsupported package manager: {}. Use 'conan' or 'vcpkg'",
				input
			),
		}
	}
}

pub trait PackageManagerGenerator {
	fn name(&self) -> &'static str;
	fn generate_manifest(&self, project_name: &str) -> String;
	fn extension(&self) -> &'static str;
	fn generate_install_instructions(&self) -> String;
}

pub struct ConanGenerator;

impl PackageManagerGenerator for ConanGenerator {
	/// The display name for the Conan package manager.
	///
	/// # Returns
	///
	/// `"Conan"`
	///
	/// # Examples
	///
	/// ```
	/// let gen = crate::ConanGenerator;
	/// assert_eq!(gen.name(), "Conan");
	/// ```
	fn name(&self) -> &'static str {
		"Conan"
	}

	/// File extension used for Conan manifest files.
	///
	/// # Returns
	///
	/// The string `"conanfile.txt"`.
	///
	/// # Examples
	///
	/// ```
	/// let gen = crate::ConanGenerator;
	/// assert_eq!(gen.extension(), "conanfile.txt");
	/// ```
	fn extension(&self) -> &'static str {
		"conanfile.txt"
	}

	/// Produces a Conan manifest template with default CMake generators and empty sections.
	///
	/// The returned template contains the `[requires]`, `[generators]` (including `CMakeDeps` and
	/// `CMakeToolchain`), `[options]`, and `[imports]` sections. The `project_name` parameter is not
	/// used in this template.
	///
	/// # Examples
	///
	/// ```
	/// let gen = crate::package_managers::ConanGenerator;
	/// let manifest = gen.generate_manifest("my_project");
	/// assert!(manifest.contains("CMakeToolchain"));
	/// assert!(manifest.contains("[requires]"));
	/// ```
	fn generate_manifest(&self, _project_name: &str) -> String {
		"[requires]\n\
		\n\
		[generators]\n\
		CMakeDeps\n\
		CMakeToolchain\n\
		\n\
		[options]\n\
		\n\
		[imports]\n"
			.to_string()
	}

	/// Provides step-by-step instructions for installing and using Conan with the project.
	///
	/// The returned string contains a multi-line set of numbered steps: installing Conan,
	/// adding dependencies to `conanfile.txt` under the `[requires]` section, running
	/// `conan install . --build=missing`, and using the generated CMake toolchain.
	///
	/// # Examples
	///
	/// ```
	/// let gen = super::ConanGenerator;
	/// let instr = gen.generate_install_instructions();
	/// assert!(instr.contains("pip install conan"));
	/// assert!(instr.contains("conan install . --build=missing"));
	/// ```
	fn generate_install_instructions(&self) -> String {
		"To use Conan with this project:\n\n\
		1. Install Conan: pip install conan\n\
		2. Add dependencies to conanfile.txt in [requires] section:\n\
		   Example: libcurl/7.85.0\n\
		3. Install dependencies: conan install . --build=missing\n\
		4. Use generated CMake toolchain in your CMakeLists.txt"
			.to_string()
	}
}

pub struct VcpkgGenerator;

impl PackageManagerGenerator for VcpkgGenerator {
	/// Display name for the vcpkg package manager.
	///
	/// # Examples
	///
	/// ```
	/// let gen = VcpkgGenerator {};
	/// assert_eq!(gen.name(), "vcpkg");
	/// ```
	fn name(&self) -> &'static str {
		"vcpkg"
	}

	/// File extension used for vcpkg manifest files.
	///
	/// # Examples
	///
	/// ```
	/// let ext = VcpkgGenerator.extension();
	/// assert_eq!(ext, "vcpkg.json");
	/// ```
	fn extension(&self) -> &'static str {
		"vcpkg.json"
	}

	/// Generates a minimal vcpkg.json manifest using the provided project name.
	///
	/// The manifest will contain the given `project_name` as the `name`, a fixed
	/// version of "0.1.0", and an empty `dependencies` array.
	///
	/// # Examples
	///
	/// ```
	/// let gen = crate::get_package_manager_generator(crate::PackageManager::Vcpkg);
	/// let manifest = gen.generate_manifest("example-project");
	/// assert!(manifest.contains("\"name\": \"example-project\""));
	/// assert!(manifest.contains("\"dependencies\": ["));
	/// ```
	fn generate_manifest(&self, project_name: &str) -> String {
		format!(
			"{{\n\
			  \"name\": \"{}\",\n\
			  \"version\": \"0.1.0\",\n\
			  \"dependencies\": [\n\
			  \n\
			  ]\n\
			}}\n",
			project_name
		)
	}

	/// Returns platform-agnostic, step-by-step instructions for installing and using vcpkg with the project.
	///
	/// The returned multi-line string describes how to clone and bootstrap vcpkg, add dependencies to
	/// `vcpkg.json`, install packages, and enable the vcpkg CMake toolchain.
	///
	/// # Examples
	///
	/// ```
	/// let v = crate::VcpkgGenerator;
	/// let instr = v.generate_install_instructions();
	/// assert!(instr.contains("Clone vcpkg"));
	/// assert!(instr.contains("vcpkg.json"));
	/// assert!(instr.contains("CMAKE_TOOLCHAIN_FILE"));
	/// ```
	fn generate_install_instructions(&self) -> String {
		"To use vcpkg with this project:\n\n\
		1. Clone vcpkg: git clone https://github.com/Microsoft/vcpkg.git\n\
		2. Run bootstrap: ./vcpkg/bootstrap-vcpkg.sh\n\
		3. Add dependencies to vcpkg.json in \"dependencies\" array:\n\
		   Example: \"libcurl:x64-linux\"\n\
		4. Install: ./vcpkg/vcpkg install\n\
		5. Use vcpkg toolchain in CMakeLists.txt:\n\
		   -DCMAKE_TOOLCHAIN_FILE=./vcpkg/scripts/buildsystems/vcpkg.cmake"
			.to_string()
	}
}

/// Returns a boxed generator implementation for the given package manager.
///
/// The returned object implements `PackageManagerGenerator` and provides
/// manifest generation and installation instructions appropriate for `pm`.
///
/// # Examples
///
/// ```
/// let gen = get_package_manager_generator(PackageManager::Conan);
/// assert_eq!(gen.name(), "Conan");
///
/// let gen = get_package_manager_generator(PackageManager::Vcpkg);
/// assert_eq!(gen.extension(), "vcpkg.json");
/// ```
pub fn get_package_manager_generator(pm: PackageManager) -> Box<dyn PackageManagerGenerator> {
	match pm {
		PackageManager::Conan => Box::new(ConanGenerator),
		PackageManager::Vcpkg => Box::new(VcpkgGenerator),
	}
}