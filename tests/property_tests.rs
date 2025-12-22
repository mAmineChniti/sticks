use proptest::prelude::*;
use serial_test::serial;
use std::env;
use std::fs;
use sticks::create_dir;

proptest! {
	#[test]
	#[serial]
	fn create_dir_handles_valid_names(name in "[a-zA-Z0-9_-]{1,32}") {
		let temp_dir = env::temp_dir().join(format!(
			"sticks_proptest_{}_{}",
			std::process::id(),
			std::time::SystemTime::now()
				.duration_since(std::time::UNIX_EPOCH)
				.unwrap()
				.as_nanos()
		));
		let original_dir = env::current_dir().unwrap();

		fs::create_dir_all(&temp_dir).unwrap();
		env::set_current_dir(&temp_dir).unwrap();

		let result = create_dir(&name);

		env::set_current_dir(&original_dir).unwrap();
		fs::remove_dir_all(&temp_dir).ok();

		prop_assert!(result.is_ok() || result.as_ref().err().unwrap().to_string().contains("already exists"));
	}
}
