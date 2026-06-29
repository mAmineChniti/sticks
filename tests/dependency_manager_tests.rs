use serial_test::serial;
use std::env;
use std::fs;
use std::path::PathBuf;
use sticks::DependencyManager;

/// Guard to restore the original working directory and clean up temp directory on drop
struct CwdGuard {
	original_dir: PathBuf,
	temp_dir: PathBuf,
}

impl CwdGuard {
	fn new(temp_dir: PathBuf) -> Self {
		let original_dir = env::current_dir().unwrap();
		fs::remove_dir_all(&temp_dir).ok();
		fs::create_dir_all(&temp_dir).unwrap();
		env::set_current_dir(&temp_dir).unwrap();
		Self {
			original_dir,
			temp_dir,
		}
	}
}

impl Drop for CwdGuard {
	fn drop(&mut self) {
		let _ = env::set_current_dir(&self.original_dir);
		let _ = fs::remove_dir_all(&self.temp_dir);
	}
}

#[test]
#[serial]
fn test_dependency_manager_list_makefile() {
	let temp_dir = env::temp_dir().join(format!(
		"sticks_test_dm_list_{}",
		std::time::SystemTime::now()
			.duration_since(std::time::UNIX_EPOCH)
			.unwrap()
			.as_nanos()
	));
	let _guard = CwdGuard::new(temp_dir);

	let makefile_content = "all: clean install-deps\n\tbuild\n\ninstall-deps:\n\tsudo apt install -y libcurl openssl\n";
	fs::write("Makefile", makefile_content).unwrap();

	let result = DependencyManager::list();
	assert!(result.is_ok());
}

#[test]
#[serial]
fn test_dependency_manager_list_makefile_pacman() {
	let temp_dir = env::temp_dir().join(format!(
		"sticks_test_dm_list_pacman_{}",
		std::time::SystemTime::now()
			.duration_since(std::time::UNIX_EPOCH)
			.unwrap()
			.as_nanos()
	));
	let _guard = CwdGuard::new(temp_dir);

	let makefile_content = "all: clean install-deps\n\tbuild\n\ninstall-deps:\n\tsudo pacman -S --noconfirm curl openssl\n";
	fs::write("Makefile", makefile_content).unwrap();

	let result = DependencyManager::list();
	assert!(result.is_ok());
}

#[test]
#[serial]
fn test_dependency_manager_list_cmake() {
	let temp_dir = env::temp_dir().join(format!(
		"sticks_test_dm_cmake_list_{}",
		std::time::SystemTime::now()
			.duration_since(std::time::UNIX_EPOCH)
			.unwrap()
			.as_nanos()
	));
	let _guard = CwdGuard::new(temp_dir);

	let cmake_content = "cmake_minimum_required(VERSION 3.15)\nproject(myapp)\nfind_package(CURL)\nfind_package(OPENSSL)\n";
	fs::write("CMakeLists.txt", cmake_content).unwrap();

	let result = DependencyManager::list();
	assert!(result.is_ok());
}

#[test]
#[serial]
fn test_dependency_manager_add_makefile() {
	let temp_dir = env::temp_dir().join(format!(
		"sticks_test_dm_add_{}",
		std::time::SystemTime::now()
			.duration_since(std::time::UNIX_EPOCH)
			.unwrap()
			.as_nanos()
	));
	let _guard = CwdGuard::new(temp_dir);

	let makefile_content = "all: clean\n\tbuild\n";
	fs::write("Makefile", makefile_content).unwrap();

	// Use packages that are likely to exist on the system
	let pm = sticks::os_detect::detect_system_package_manager();
	let deps = match pm {
		sticks::os_detect::SystemPackageManager::Pacman => vec!["curl".to_string()],
		sticks::os_detect::SystemPackageManager::Apt => vec!["libcurl4".to_string()],
		sticks::os_detect::SystemPackageManager::Dnf => vec!["libcurl".to_string()],
		_ => vec!["curl".to_string()], // fallback
	};

	let result = DependencyManager::add(&deps);
	if let Err(e) = &result {
		eprintln!("Error adding dependencies: {:?}", e);
		// If package validation fails, skip the rest of the test
		if e.to_string().contains("not found") {
			return;
		}
	}
	assert!(result.is_ok());
	assert_eq!(result.unwrap().len(), 1);

	let updated = fs::read_to_string("Makefile").unwrap();
	for dep in &deps {
		assert!(updated.contains(dep));
	}
	assert!(updated.contains("install-deps"));
}

#[test]
#[serial]
fn test_dependency_manager_add_cmake() {
	let temp_dir = env::temp_dir().join(format!(
		"sticks_test_dm_add_cmake_{}",
		std::time::SystemTime::now()
			.duration_since(std::time::UNIX_EPOCH)
			.unwrap()
			.as_nanos()
	));
	let _guard = CwdGuard::new(temp_dir);

	let cmake_content = "cmake_minimum_required(VERSION 3.15)\nproject(myapp)\n";
	fs::write("CMakeLists.txt", cmake_content).unwrap();

	let result = DependencyManager::add(&["CURL".to_string()]);
	assert!(result.is_ok());
	assert_eq!(result.unwrap().len(), 1);

	let updated = fs::read_to_string("CMakeLists.txt").unwrap();
	assert!(updated.contains("CURL"));
}

#[test]
#[serial]
fn test_dependency_manager_remove_makefile() {
	let temp_dir = env::temp_dir().join(format!(
		"sticks_test_dm_remove_{}",
		std::time::SystemTime::now()
			.duration_since(std::time::UNIX_EPOCH)
			.unwrap()
			.as_nanos()
	));
	let _guard = CwdGuard::new(temp_dir);

	let makefile_content = "all: clean install-deps\n\tbuild\n\ninstall-deps:\n\tsudo apt install -y libcurl openssl\n";
	fs::write("Makefile", makefile_content).unwrap();

	let result = DependencyManager::remove(&["libcurl".to_string()]);
	assert!(result.is_ok());
	assert_eq!(result.unwrap().len(), 1);

	let updated = fs::read_to_string("Makefile").unwrap();
	assert!(!updated.contains("libcurl"));
	assert!(updated.contains("openssl"));
}

#[test]
#[serial]
fn test_dependency_manager_no_build_system() {
	let temp_dir = env::temp_dir().join(format!(
		"sticks_test_dm_none_{}",
		std::time::SystemTime::now()
			.duration_since(std::time::UNIX_EPOCH)
			.unwrap()
			.as_nanos()
	));
	let _guard = CwdGuard::new(temp_dir);

	// No build system present
	let result = DependencyManager::add(&["libcurl".to_string()]);
	assert!(result.is_err());
	let error_msg = result.unwrap_err().to_string();
	assert!(error_msg.contains("No build system") || error_msg.contains("not found"));
}
