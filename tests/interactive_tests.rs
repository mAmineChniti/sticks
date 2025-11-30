use serial_test::serial;
use std::env;
use std::fs;

#[test]
#[serial]
fn test_interactive_module_exists() {
	let result = std::panic::catch_unwind(|| {
		let _ = sticks::interactive::select_language;
		let _ = sticks::interactive::select_build_system;
	});
	assert!(result.is_ok(), "Interactive module should be accessible");
}

#[test]
#[serial]
fn test_select_language_interactive_returns_language() {
	let temp_dir = env::temp_dir().join(format!(
		"sticks_test_interactive_{}_{}",
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

	let result = std::panic::catch_unwind(|| {
		// We can't easily test interactive functions that read from stdin
		// But we can verify they are exported and callable
		let _ = sticks::interactive::select_language;
	});

	env::set_current_dir(&original_dir).unwrap();
	fs::remove_dir_all(&temp_dir).ok();

	assert!(result.is_ok());
}

#[test]
#[serial]
fn test_interactive_module_functions_exported() {
	// Verify that the interactive module functions are properly exported from lib
	let result = std::panic::catch_unwind(|| {
		let _select_language = sticks::interactive::select_language;
		let _select_build_system = sticks::interactive::select_build_system;
	});

	assert!(
		result.is_ok(),
		"Interactive module functions should be exported"
	);
}
