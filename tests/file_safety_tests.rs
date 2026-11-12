use serial_test::serial;
use std::fs;
use std::path::Path;
use sticks::file_handler::{self, backup_path};

fn tempdir(tag: &str) -> tempfile::TempDir {
	tempfile::tempdir().unwrap_or_else(|_| panic!("tempdir {tag}"))
}

#[test]
#[serial]
fn write_atomic_backs_up_before_overwrite() {
	let dir = tempdir("backup");
	let file = dir.path().join("Makefile");
	fs::write(&file, "old content\n").unwrap();

	file_handler::write_atomic(&file, "new content\n").unwrap();

	assert_eq!(fs::read_to_string(&file).unwrap(), "new content\n");
	assert_eq!(
		fs::read_to_string(backup_path(&file)).unwrap(),
		"old content\n"
	);
}

#[test]
#[serial]
fn write_atomic_is_idempotent() {
	let dir = tempdir("idem");
	let file = dir.path().join("vcpkg.json");
	fs::write(&file, "{}\n").unwrap();

	file_handler::write_atomic(&file, "{\"a\":1}\n").unwrap();
	let backup_after_first = fs::read_to_string(backup_path(&file)).unwrap();

	// Second identical write: no change, backup must not be rewritten.
	file_handler::write_atomic(&file, "{\"a\":1}\n").unwrap();
	assert_eq!(
		fs::read_to_string(backup_path(&file)).unwrap(),
		backup_after_first
	);
}

#[test]
#[serial]
fn dry_run_writes_nothing() {
	file_handler::set_dry_run(true);
	let dir = tempdir("dryrun");
	let file = dir.path().join("CMakeLists.txt");
	fs::write(&file, "old\n").unwrap();

	file_handler::write_atomic(&file, "new\n").unwrap();
	file_handler::write_new_file(&dir.path().join("fresh.txt"), "x\n").unwrap();

	assert_eq!(fs::read_to_string(&file).unwrap(), "old\n");
	assert!(!dir.path().join("fresh.txt").exists());
	assert!(!backup_path(&file).exists());
	file_handler::set_dry_run(false);
}

#[test]
#[serial]
fn force_allows_overwrite_with_backup() {
	let dir = tempdir("force");
	let file = dir.path().join("existing.txt");
	fs::write(&file, "user data\n").unwrap();

	// Without force this refuses.
	assert!(file_handler::write_new_file(&file, "new\n").is_err());

	file_handler::set_force(true);
	file_handler::write_new_file(&file, "new\n").unwrap();
	file_handler::set_force(false);

	assert_eq!(fs::read_to_string(&file).unwrap(), "new\n");
	assert_eq!(
		fs::read_to_string(backup_path(&file)).unwrap(),
		"user data\n"
	);
}

#[test]
#[serial]
fn remove_with_backup_preserves_content() {
	let dir = tempdir("rmbackup");
	let file = dir.path().join("conanfile.txt");
	fs::write(&file, "[requires]\nfmt/1.0\n").unwrap();

	file_handler::remove_with_backup(&file).unwrap();

	assert!(!file.exists());
	assert_eq!(
		fs::read_to_string(backup_path(&file)).unwrap(),
		"[requires]\nfmt/1.0\n"
	);
}

#[test]
#[serial]
fn eject_moves_sticks_files_to_backups() {
	let dir = tempdir("eject");
	for name in [
		".clang-format",
		".editorconfig",
		".gitattributes",
		"CMakePresets.json",
		"CMakeLists.txt",
		"Makefile",
	] {
		fs::write(dir.path().join(name), "content\n").unwrap();
	}
	fs::create_dir(dir.path().join(".vscode")).unwrap();
	fs::write(dir.path().join(".vscode/settings.json"), "{}").unwrap();

	sticks::eject_project_in(dir.path()).unwrap();

	for name in [
		".clang-format",
		".editorconfig",
		".gitattributes",
		"CMakePresets.json",
		".vscode",
	] {
		assert!(!dir.path().join(name).exists(), "{name} should be ejected");
		assert!(
			dir.path().join(format!("{name}.sticks.bak")).exists()
				|| dir
					.path()
					.join(format!("{}{}", name, file_handler::BACKUP_SUFFIX))
					.exists(),
			"{name} backup missing"
		);
	}
	// Build files are plain project files: untouched.
	assert!(dir.path().join("CMakeLists.txt").exists());
	assert!(dir.path().join("Makefile").exists());
}

#[test]
fn backup_path_appends_suffix() {
	assert_eq!(
		backup_path(Path::new("/tmp/proj/Makefile")),
		Path::new("/tmp/proj/Makefile.sticks.bak")
	);
}
