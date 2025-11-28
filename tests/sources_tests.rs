use serial_test::serial;
use std::env;
use std::fs;
use std::path::Path;
use sticks::add_sources;

#[test]
#[serial]
fn test_add_sources_no_src_dir() {
	let temp_dir = env::temp_dir().join(format!(
		"sticks_test_src_{}_{}",
		std::process::id(),
		std::time::SystemTime::now()
			.duration_since(std::time::UNIX_EPOCH)
			.unwrap()
			.as_nanos()
	));
	let original_dir = env::current_dir().unwrap();

	fs::remove_dir_all(&temp_dir).ok();
	fs::create_dir_all(&temp_dir).unwrap();
	env::set_current_dir(&temp_dir).unwrap();

	let result = add_sources(&["utils"]);
	assert!(result.is_err());
	assert!(result
		.unwrap_err()
		.to_string()
		.contains("src directory not found"));

	env::set_current_dir(&original_dir).unwrap();
	fs::remove_dir_all(&temp_dir).ok();
}

#[test]
#[serial]
fn test_add_sources_success() {
	let temp_dir = env::temp_dir().join(format!(
		"sticks_test_add_src_{}_{}",
		std::process::id(),
		std::time::SystemTime::now()
			.duration_since(std::time::UNIX_EPOCH)
			.unwrap()
			.as_nanos()
	));
	let original_dir = env::current_dir().unwrap();

	fs::remove_dir_all(&temp_dir).ok();
	fs::create_dir_all(&temp_dir).unwrap();
	env::set_current_dir(&temp_dir).unwrap();
	fs::create_dir("src").unwrap();
	fs::write("src/main.c", "int main() {}").unwrap();

	let result = add_sources(&["utils", "network"]);
	assert!(result.is_ok());

	assert!(
		Path::new("src/utils.c").exists(),
		"src/utils.c should exist"
	);
	assert!(
		Path::new("src/utils.h").exists(),
		"src/utils.h should exist"
	);
	assert!(
		Path::new("src/network.c").exists(),
		"src/network.c should exist"
	);
	assert!(
		Path::new("src/network.h").exists(),
		"src/network.h should exist"
	);

	let header_content = fs::read_to_string("src/utils.h").unwrap();
	assert!(header_content.contains("#ifndef UTILS_H"));
	assert!(header_content.contains("#define UTILS_H"));
	assert!(header_content.contains("#endif"));

	env::set_current_dir(&original_dir).unwrap();
	fs::remove_dir_all(&temp_dir).ok();
}
