use sticks::create_dir;
use std::env;
use std::fs;
use serial_test::serial;

#[test]
#[serial]
fn test_create_dir_success() {
	let temp_dir = env::temp_dir().join(format!("sticks_test_{}_{}", std::process::id(), std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
	let test_dir_name = "test_project";
	let original_dir = env::current_dir().unwrap();

	fs::remove_dir_all(&temp_dir).ok();
	fs::create_dir_all(&temp_dir).unwrap();
	env::set_current_dir(&temp_dir).unwrap();

	let result = create_dir(test_dir_name);
	assert!(result.is_ok());

	let created_path = temp_dir.join(test_dir_name);
	assert!(created_path.exists());

	env::set_current_dir(&original_dir).unwrap();
	fs::remove_dir_all(&temp_dir).ok();
}

#[test]
#[serial]
fn test_create_dir_already_exists() {
	let temp_dir = env::temp_dir().join(format!("sticks_test_exists_{}_{}", std::process::id(), std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
	let test_dir_name = "existing_project";
	let original_dir = env::current_dir().unwrap();

	fs::remove_dir_all(&temp_dir).ok();
	fs::create_dir_all(&temp_dir).unwrap();
	env::set_current_dir(&temp_dir).unwrap();
	fs::create_dir(test_dir_name).unwrap();

	let result = create_dir(test_dir_name);
	assert!(result.is_err());
	assert!(result.unwrap_err().to_string().contains("already exists"));

	env::set_current_dir(&original_dir).unwrap();
	fs::remove_dir_all(&temp_dir).ok();
}
