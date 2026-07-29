use std::path::Path;

use super::*;

#[test]
fn creates_the_file_with_the_given_contents() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("config.toml");

    write_atomic(&path, b"a = 1\n", None).unwrap();

    assert_eq!(std::fs::read_to_string(&path).unwrap(), "a = 1\n");
}

#[test]
fn replaces_existing_contents_without_truncating_first() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("config.toml");
    std::fs::write(&path, "old = true\n").unwrap();

    write_atomic(&path, b"new = true\n", None).unwrap();

    assert_eq!(std::fs::read_to_string(&path).unwrap(), "new = true\n");
}

#[test]
fn leaves_no_temporary_file_behind() {
    // The tempfile lives in the target's own directory so the rename stays
    // within one filesystem; it must not survive the call.
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("config.toml");

    write_atomic(&path, b"x = 1\n", None).unwrap();

    let entries: Vec<_> = std::fs::read_dir(tmp.path())
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .collect();
    assert_eq!(entries, vec![std::ffi::OsString::from("config.toml")]);
}

#[cfg(unix)]
#[test]
fn applies_the_requested_mode() {
    use std::os::unix::fs::PermissionsExt as _;

    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("secret");

    write_atomic(&path, b"token", Some(0o600)).unwrap();

    let mode = std::fs::metadata(&path).unwrap().permissions().mode();
    assert_eq!(mode & 0o777, 0o600, "mode was {mode:o}");
}

#[cfg(unix)]
#[test]
fn a_replacement_never_appears_at_the_default_mode() {
    // The mode is applied before the rename precisely so the target is not
    // briefly world-readable. Replacing an existing 0o600 file with a
    // 0o600 write must never widen it, even transiently — the strongest
    // assertion available without racing the syscall is that the final
    // mode is the requested one and not the umask default.
    use std::os::unix::fs::PermissionsExt as _;

    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("secret");
    std::fs::write(&path, b"old").unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();

    write_atomic(&path, b"new", Some(0o600)).unwrap();

    let mode = std::fs::metadata(&path).unwrap().permissions().mode();
    assert_eq!(mode & 0o777, 0o600, "mode was {mode:o}");
    assert_eq!(std::fs::read(&path).unwrap(), b"new");
}

#[test]
fn omitting_the_mode_leaves_the_platform_default() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("plain");

    write_atomic(&path, b"x", None).unwrap();

    assert!(std::fs::metadata(&path).unwrap().is_file());
}

#[test]
fn a_missing_directory_is_reported_not_created() {
    // WHY assert the negative too: silently creating the directory would
    // decide its permission mode, which is the caller's to decide — a
    // secret store wants 0o700 and would not get it.
    let tmp = tempfile::tempdir().unwrap();
    let missing = tmp.path().join("absent");
    let path = missing.join("config.toml");

    let error = write_atomic(&path, b"x", None).unwrap_err();

    assert!(
        matches!(error, AtomicWriteError::Write { .. }),
        "expected a write error, got {error:?}"
    );
    assert!(
        !missing.exists(),
        "the directory must not have been created"
    );
}

#[test]
fn a_bare_filename_resolves_to_the_current_directory() {
    // `Path::parent` of a bare filename is Some(""), not None, so guarding
    // only the None case leaves an empty path that no tempfile can be
    // created in. This is the assertion that distinguishes the two.
    assert_eq!(Path::new("bare.toml").parent(), Some(Path::new("")));
    assert_eq!(target_dir(Path::new("bare.toml")), Path::new("."));
    assert_eq!(target_dir(Path::new("/etc/hosts")), Path::new("/etc"));
    assert_eq!(target_dir(Path::new("/")), Path::new("."));
}
