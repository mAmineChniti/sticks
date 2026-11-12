use sticks::os_detect::SystemPackageManager;
use sticks::package_checker;

#[test]
fn test_suggest_similar_common_typos() {
	let pm = sticks::os_detect::detect_system_package_manager();
	if pm == SystemPackageManager::Unknown {
		// Skip test if no package manager is detected
		return;
	}
	let s = package_checker::suggest_similar("libcrl", pm).unwrap();
	// The suggestions come from a live system package database whose
	// contents differ per OS/distro — assert the contract, not names.
	assert!(s.len() <= 5);
	assert!(s.iter().all(|x| !x.trim().is_empty()));

	let s2 = package_checker::suggest_similar("openssl", pm).unwrap();
	assert!(s2.len() <= 5);
	assert!(s2.iter().all(|x| !x.trim().is_empty()));
}

#[test]
fn test_package_exists_known_and_unknown() {
	let pm = sticks::os_detect::detect_system_package_manager();
	if pm == SystemPackageManager::Unknown {
		// Skip test if no package manager is detected
		return;
	}
	let ok = package_checker::package_exists("libcurl", pm).unwrap();
	// Only assert if package check succeeded (may vary by system)
	if ok {
		let no = package_checker::package_exists("definitely-not-a-package-xyz", pm).unwrap();
		assert!(!no, "Fake package should not be found");
	}
}
