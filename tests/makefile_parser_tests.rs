use serial_test::serial;
use std::env;
use std::fs;
use sticks::makefile_parser::Makefile;

#[test]
#[serial]
fn test_makefile_parser_basic() {
	let content = "all: clean\n\tbuild\n\nclean:\n\trm -f *.o\n";
	let makefile = Makefile::parse(content).unwrap();

	assert!(makefile.rules.contains_key("all"));
	assert!(makefile.rules.contains_key("clean"));

	let all_rule = &makefile.rules["all"];
	assert_eq!(all_rule.target, "all");
	assert_eq!(all_rule.dependencies, vec!["clean"]);
	assert_eq!(all_rule.commands.len(), 1);
}

#[test]
#[serial]
fn test_makefile_add_dependencies_basic() {
	let temp_dir = env::temp_dir().join(format!(
		"sticks_test_makefile_add_{}",
		std::time::SystemTime::now()
			.duration_since(std::time::UNIX_EPOCH)
			.unwrap()
			.as_nanos()
	));
	let original_dir = env::current_dir().unwrap();

	fs::remove_dir_all(&temp_dir).ok();
	fs::create_dir_all(&temp_dir).unwrap();
	env::set_current_dir(&temp_dir).unwrap();

	let content = "all: clean\n\tbuild\n";
	let mut makefile = Makefile::parse(content).unwrap();

	let added = makefile.add_dependencies(&["libcurl".to_string()]).unwrap();
	assert_eq!(added.len(), 1);
	assert_eq!(added[0], "libcurl");

	assert!(makefile.rules.contains_key("install-deps"));
	let install_deps = &makefile.rules["install-deps"];
	assert!(!install_deps.commands.is_empty());
	assert!(install_deps.commands[0].contains("libcurl"));

	env::set_current_dir(&original_dir).unwrap();
	fs::remove_dir_all(&temp_dir).ok();
}

#[test]
#[serial]
fn test_makefile_add_multiple_dependencies() {
	let content = "all: clean\n\tbuild\n";
	let mut makefile = Makefile::parse(content).unwrap();

	let added = makefile
		.add_dependencies(&[
			"libcurl".to_string(),
			"openssl".to_string(),
			"libssl-dev".to_string(),
		])
		.unwrap();

	assert_eq!(added.len(), 3);

	let install_deps = &makefile.rules["install-deps"];
	let cmd = &install_deps.commands[0];
	assert!(cmd.contains("libcurl"));
	assert!(cmd.contains("openssl"));
	assert!(cmd.contains("libssl-dev"));

	// Should be sorted
	let libcurl_pos = cmd.find("libcurl").unwrap();
	let openssl_pos = cmd.find("openssl").unwrap();
	assert!(libcurl_pos < openssl_pos);
}

#[test]
#[serial]
fn test_makefile_deduplicate_dependencies() {
	let content =
		"all: clean install-deps\n\tbuild\n\ninstall-deps:\n\tsudo apt install -y libcurl\n";
	let mut makefile = Makefile::parse(content).unwrap();

	let added = makefile.add_dependencies(&["libcurl".to_string()]).unwrap();
	assert_eq!(added.len(), 0); // Already present, not added again
}

#[test]
#[serial]
fn test_makefile_remove_dependencies() {
	let content = "all: clean install-deps\n\tbuild\n\ninstall-deps:\n\tsudo apt install -y libcurl openssl libssl-dev\n";
	let mut makefile = Makefile::parse(content).unwrap();

	let removed = makefile
		.remove_dependencies(&["openssl".to_string()])
		.unwrap();
	assert_eq!(removed.len(), 1);

	let install_deps = &makefile.rules["install-deps"];
	let cmd = &install_deps.commands[0];
	assert!(cmd.contains("libcurl"));
	assert!(cmd.contains("libssl-dev"));
	assert!(!cmd.contains("openssl"));
}

#[test]
#[serial]
fn test_makefile_remove_all_dependencies() {
	let content =
		"all: clean install-deps\n\tbuild\n\ninstall-deps:\n\tsudo apt install -y libcurl\n";
	let mut makefile = Makefile::parse(content).unwrap();

	let removed = makefile
		.remove_dependencies(&["libcurl".to_string()])
		.unwrap();
	assert_eq!(removed.len(), 1);

	// install-deps rule should be removed entirely
	assert!(!makefile.rules.contains_key("install-deps"));
}

#[test]
#[serial]
fn test_makefile_validate_dependency_names() {
	let content = "all: clean\n\tbuild\n";
	let mut makefile = Makefile::parse(content).unwrap();

	// Valid names should work
	assert!(makefile.add_dependencies(&["libcurl".to_string()]).is_ok());
	assert!(
		makefile
			.add_dependencies(&["lib-curl_2.0".to_string()])
			.is_ok()
	);

	// Invalid names should fail
	let mut makefile2 = Makefile::parse(content).unwrap();
	assert!(
		makefile2
			.add_dependencies(&["lib;curl".to_string()])
			.is_err()
	);
	assert!(
		makefile2
			.add_dependencies(&["lib curl".to_string()])
			.is_err()
	);
}

#[test]
#[serial]
fn test_makefile_roundtrip_serialization() {
	let content = "all: clean\n\tbuild\n\nclean:\n\trm -f *.o\n";
	let makefile = Makefile::parse(content).unwrap();

	let serialized = makefile.to_makefile_string();
	assert!(serialized.contains("all: clean"));
	assert!(serialized.contains("clean:"));
}

#[test]
#[serial]
fn test_makefile_write_to_file() {
	let temp_dir = env::temp_dir().join(format!(
		"sticks_test_makefile_write_{}",
		std::time::SystemTime::now()
			.duration_since(std::time::UNIX_EPOCH)
			.unwrap()
			.as_nanos()
	));
	let original_dir = env::current_dir().unwrap();

	fs::remove_dir_all(&temp_dir).ok();
	fs::create_dir_all(&temp_dir).unwrap();
	env::set_current_dir(&temp_dir).unwrap();

	let content = "all: clean\n\tbuild\n";
	let mut makefile = Makefile::parse(content).unwrap();
	makefile.add_dependencies(&["libcurl".to_string()]).unwrap();

	makefile.write_to_file("Makefile").unwrap();

	let written = fs::read_to_string("Makefile").unwrap();
	assert!(written.contains("libcurl"));
	assert!(written.contains("install-deps"));

	env::set_current_dir(&original_dir).unwrap();
	fs::remove_dir_all(&temp_dir).ok();
}
