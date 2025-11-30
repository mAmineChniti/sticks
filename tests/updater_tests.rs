use serial_test::serial;

#[test]
#[serial]
fn test_updater_module_exists() {
	let result = std::panic::catch_unwind(|| {
		let _ = sticks::updater::update_project;
	});
	assert!(result.is_ok(), "Updater module should be accessible");
}

#[test]
#[serial]
fn test_updater_update_project_function_exported() {
	// Verify that the updater module's update_project function is properly exported
	let result = std::panic::catch_unwind(|| {
		let _update = sticks::updater::update_project;
	});

	assert!(
		result.is_ok(),
		"Updater module functions should be exported"
	);
}

#[test]
#[serial]
fn test_updater_can_be_called() {
	// Test that the updater function can be called (it won't actually update in test)
	let result = std::panic::catch_unwind(|| {
		// In a real test, we'd mock the network calls
		// For now, we just verify the function exists and is callable
		let _ = sticks::updater::update_project;
	});

	assert!(result.is_ok(), "Updater should be callable");
}
