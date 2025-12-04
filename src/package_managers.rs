use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PackageManager {
	Conan,
	Vcpkg,
}

impl std::fmt::Display for PackageManager {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			PackageManager::Conan => write!(f, "Conan"),
			PackageManager::Vcpkg => write!(f, "vcpkg"),
		}
	}
}

impl FromStr for PackageManager {
	type Err = anyhow::Error;

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
	fn name(&self) -> &'static str {
		"Conan"
	}

	fn extension(&self) -> &'static str {
		"conanfile.txt"
	}

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
	fn name(&self) -> &'static str {
		"vcpkg"
	}

	fn extension(&self) -> &'static str {
		"vcpkg.json"
	}

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

pub fn get_package_manager_generator(pm: PackageManager) -> Box<dyn PackageManagerGenerator> {
	match pm {
		PackageManager::Conan => Box::new(ConanGenerator),
		PackageManager::Vcpkg => Box::new(VcpkgGenerator),
	}
}
