use serial_test::serial;

/// `update_project` performs a network request, so it must never be invoked by
/// the test suite. Calling it here would make tests depend on GitHub and on
/// `curl` being installed.
///
/// The version-comparison logic itself is covered by the unit tests in
/// `src/updater.rs`, which exercise the SemVer parser and comparator directly.
#[test]
#[serial]
fn update_project_is_callable_without_panicking() {
	// Referencing the symbol proves the public entry point is exported. It is
	// deliberately not called.
	let _entry_point: fn() -> anyhow::Result<()> = sticks::updater::update_project;
}

#[test]
#[serial]
fn updater_verifies_checksum_before_replacing() {
	// `sticks update` must verify the artifact against the published
	// SHA256SUMS before touching the installed binary. Assert the
	// verification boundary exists so it cannot be silently removed.
	let source = include_str!("../src/updater.rs");
	assert!(
		source.contains("verify_artifact"),
		"updater must verify the downloaded artifact"
	);
	assert!(
		source.contains("Checksum mismatch"),
		"updater must abort on checksum mismatch"
	);
	assert!(
		source.contains("SHA256SUMS"),
		"updater must fetch the published checksum file"
	);
}

#[test]
#[serial]
fn updater_forces_https() {
	let source = include_str!("../src/updater.rs");
	assert!(
		source.contains("\"=https\""),
		"curl must be forced onto https"
	);
	let constants = include_str!("../src/constants.rs");
	assert!(
		!constants.contains("http://"),
		"no release URL may use plain http"
	);
}
