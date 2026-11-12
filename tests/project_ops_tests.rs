use sticks::project_ops::{generate_cmake_presets, merge_cmake_presets};

#[test]
fn fresh_presets_contain_debug_release_build_test() {
	let content = generate_cmake_presets(false, false);
	let root: serde_json::Value = serde_json::from_str(&content).unwrap();
	for array in ["configurePresets", "buildPresets", "testPresets"] {
		let names: Vec<&str> = root[array]
			.as_array()
			.unwrap()
			.iter()
			.filter_map(|p| p["name"].as_str())
			.collect();
		assert!(names.contains(&"debug"), "{array} missing debug: {names:?}");
		assert!(
			names.contains(&"release"),
			"{array} missing release: {names:?}"
		);
	}
	assert!(!content.contains("conan"));
}

#[test]
fn fresh_presets_include_conan_when_requested() {
	let content = generate_cmake_presets(true, false);
	assert!(content.contains("conan_toolchain.cmake"));
}

#[test]
fn conan_debug_release_inherit_hidden_base() {
	// The hidden `conan` preset carries the toolchain; debug/release must
	// inherit it or `cmake --preset debug` can't find conan_toolchain.cmake.
	let value: serde_json::Value =
		serde_json::from_str(&generate_cmake_presets(true, false)).unwrap();
	let configure = value["configurePresets"].as_array().unwrap();
	let conan = configure
		.iter()
		.find(|p| p["name"] == "conan")
		.expect("hidden conan base preset");
	assert_eq!(conan["hidden"], true);
	for name in ["debug", "release"] {
		let preset = configure
			.iter()
			.find(|p| p["name"] == name)
			.unwrap_or_else(|| panic!("missing {name} preset"));
		assert_eq!(preset["inherits"], "conan", "{name} must inherit conan");
	}
}

#[test]
fn ninja_flag_adds_generator() {
	let with = generate_cmake_presets(false, true);
	let without = generate_cmake_presets(false, false);
	assert!(with.contains("\"generator\": \"Ninja\""));
	assert!(!without.contains("\"generator\""));
}

#[test]
fn merge_preserves_user_presets() {
	let existing = r#"{
		"version": 6,
		"configurePresets": [{"name": "my-custom", "generator": "Ninja"}]
	}"#;
	let merged = merge_cmake_presets(existing, false, false).unwrap();
	assert!(merged.contains("my-custom"));
	assert!(merged.contains("\"debug\""));
	assert!(merged.contains("\"release\""));
}

#[test]
fn merge_does_not_duplicate_existing_names() {
	let existing = generate_cmake_presets(false, false);
	let merged = merge_cmake_presets(&existing, false, false).unwrap();
	assert_eq!(merged, existing);
}

#[test]
fn merge_adds_conan_preset_to_existing_file() {
	let existing = r#"{"version": 6, "configurePresets": []}"#;
	let merged = merge_cmake_presets(existing, true, false).unwrap();
	assert!(merged.contains("conan_toolchain.cmake"));
}

#[test]
fn merge_rejects_invalid_json() {
	assert!(merge_cmake_presets("not json", false, false).is_err());
}
