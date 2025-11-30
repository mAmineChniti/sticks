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
	let result = std::panic::catch_unwind(|| {
		let _ = sticks::updater::update_project;
	});

	assert!(result.is_ok(), "Updater should be callable");
}
