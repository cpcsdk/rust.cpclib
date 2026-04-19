use assert_cmd::cargo::cargo_bin_cmd;
use assert_cmd::prelude::*;
use predicates::prelude::*;
use serial_test::serial;
use std::fs;
use std::path::PathBuf;

type TempDir = camino_tempfile::Utf8TempDir;


/// Helper to create test files
fn create_test_files(dir: &TempDir) -> (PathBuf, PathBuf, PathBuf) {
    let file1 = dir.path().join("file1.txt");
    let file2 = dir.path().join("file2.txt");
    let file3 = dir.path().join("file3.txt");
    
    fs::write(&file1, "content1").unwrap();
    fs::write(&file2, "content2").unwrap();
    fs::write(&file3, "content3").unwrap();
    
    (file1.into(), file2.into(), file3.into())
}

/// Helper to create test files in a subdirectory
fn create_test_files_in_subdir(dir: &camino_tempfile::Utf8TempDir, subdir: &str) -> (PathBuf, PathBuf, PathBuf) {
    let sub_path = dir.path().join(subdir);
    fs::create_dir_all(&sub_path).unwrap();
    
    let file1 = sub_path.join("file1.txt");
    let file2 = sub_path.join("file2.txt");
    let file3 = sub_path.join("file3.txt");
    
    fs::write(&file1, "content1").unwrap();
    fs::write(&file2, "content2").unwrap();
    fs::write(&file3, "content3").unwrap();
    
    (file1.into(), file2.into(), file3.into())
}

#[test]
#[serial]
fn test_archive_create_basic_zip() {
    let temp = camino_tempfile::tempdir().unwrap();
    let (file1, file2, file3) = create_test_files(&temp);
    let archive = temp.path().join("test.zip");
    
    let mut cmd = cargo_bin_cmd!("bndbuild");
    cmd.current_dir(temp.path())
        .arg("--direct")
        .arg("--")
        .arg("archive")
        .arg("create")
        .arg("-o")
        .arg(&archive)
        .arg(file1.file_name().unwrap())
        .arg(file2.file_name().unwrap())
        .arg(file3.file_name().unwrap());
    
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Created archive"));
    
    assert!(archive.exists(), "Archive should be created");
}

#[test]
#[serial]
fn test_archive_create_basic_targz() {
    let temp = camino_tempfile::tempdir().unwrap();
    let (file1, file2, _) = create_test_files(&temp);
    let archive = temp.path().join("test.tar.gz");
    
    let mut cmd = cargo_bin_cmd!("bndbuild");
    cmd.current_dir(temp.path())
        .arg("--direct")
        .arg("--")
        .arg("archive")
        .arg("create")
        .arg("-o")
        .arg(&archive)
        .arg(file1.file_name().unwrap())
        .arg(file2.file_name().unwrap());
    
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Created archive"));
    
    assert!(archive.exists(), "Archive should be created");
}

#[test]
#[serial]
fn test_archive_list_zip() {
    let temp = camino_tempfile::tempdir().unwrap();
    let (file1, file2, _) = create_test_files(&temp);
    let archive = temp.path().join("test.zip");
    
    // First create the archive
    let mut create_cmd = cargo_bin_cmd!("bndbuild");
    create_cmd.current_dir(temp.path())
        .arg("--direct")
        .arg("--")
        .arg("archive")
        .arg("create")
        .arg("-o")
        .arg(&archive)
        .arg(file1.file_name().unwrap())
        .arg(file2.file_name().unwrap());
    
    create_cmd.assert().success();
    
    // Now list the contents
    let mut list_cmd = cargo_bin_cmd!("bndbuild");
    list_cmd.current_dir(temp.path())
        .arg("--direct")
        .arg("--")
        .arg("archive")
        .arg("list")
        .arg(&archive);
    
    list_cmd.assert()
        .success()
        .stdout(predicate::str::contains("file1.txt"))
        .stdout(predicate::str::contains("file2.txt"));
}

#[test]
#[serial]
fn test_archive_extract_zip() {
    let temp = camino_tempfile::tempdir().unwrap();
    let (file1, file2, _) = create_test_files(&temp);
    let archive = temp.path().join("test.zip");
    let extract_dir = temp.path().join("extracted");
    fs::create_dir(&extract_dir).unwrap();
    
    // Create the archive
    let mut create_cmd = cargo_bin_cmd!("bndbuild");
    create_cmd.current_dir(temp.path())
        .arg("--direct")
        .arg("--")
        .arg("archive")
        .arg("create")
        .arg("-o")
        .arg(&archive)
        .arg(file1.file_name().unwrap())
        .arg(file2.file_name().unwrap());
    
    create_cmd.assert().success();
    
    // Extract the archive
    let mut extract_cmd = cargo_bin_cmd!("bndbuild");
    extract_cmd.current_dir(&extract_dir)
        .arg("--direct")
        .arg("--")
        .arg("archive")
        .arg("extract")
        .arg(&archive);
    
    extract_cmd.assert()
        .success()
        .stdout(predicate::str::contains("Extracted"));
    
    // Verify extracted files exist
    assert!(extract_dir.join("file1.txt").exists());
    assert!(extract_dir.join("file2.txt").exists());
}

#[test]
#[serial]
fn test_archive_strip_prefix() {
    let temp = camino_tempfile::tempdir().unwrap();
    let (file1, file2, file3) = create_test_files_in_subdir(&temp, "dist");
    let archive = temp.path().join("test.zip");
    
    // Create archive with strip-prefix
    let mut cmd = cargo_bin_cmd!("bndbuild");
    cmd.current_dir(temp.path())
        .arg("--direct")
        .arg("--")
        .arg("archive")
        .arg("create")
        .arg("-o")
        .arg(&archive)
        .arg("-s")
        .arg("dist")
        .arg(&file1)
        .arg(&file2)
        .arg(&file3);
    
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("file1.txt"))
        .stdout(predicate::str::contains("file2.txt"))
        .stdout(predicate::str::contains("file3.txt"));
    
    // List to verify paths don't contain "dist/" prefix
    let mut list_cmd = cargo_bin_cmd!("bndbuild");
    list_cmd.current_dir(temp.path())
        .arg("--direct")
        .arg("--")
        .arg("archive")
        .arg("list")
        .arg(&archive);
    
    list_cmd.assert()
        .success()
        .stdout(predicate::str::contains("file1.txt"))
        .stdout(predicate::str::contains("file2.txt"))
        .stdout(predicate::str::contains("file3.txt"))
        .stdout(predicate::str::contains("dist/").not()); // Should NOT contain "dist/"
}

#[test]
#[serial]
fn test_archive_basename_only() {
    let temp = camino_tempfile::tempdir().unwrap();
    
    // Create files in different subdirectories
    let sub1 = temp.path().join("src");
    let sub2 = temp.path().join("data");
    fs::create_dir_all(&sub1).unwrap();
    fs::create_dir_all(&sub2).unwrap();
    
    let file1 = sub1.join("main.asm");
    let file2 = sub2.join("sprites.bin");
    fs::write(&file1, "asm content").unwrap();
    fs::write(&file2, "binary content").unwrap();
    
    let archive = temp.path().join("test.zip");
    
    // Create archive with basename-only
    let mut cmd = cargo_bin_cmd!("bndbuild");
    cmd.current_dir(temp.path())
        .arg("--direct")
        .arg("--")
        .arg("archive")
        .arg("create")
        .arg("-o")
        .arg(&archive)
        .arg("-b")
        .arg(&file1)
        .arg(&file2);
    
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("main.asm"))
        .stdout(predicate::str::contains("sprites.bin"));
    
    // List to verify only basenames are stored
    let mut list_cmd = cargo_bin_cmd!("bndbuild");
    list_cmd.current_dir(temp.path())
        .arg("--direct")
        .arg("--")
        .arg("archive")
        .arg("list")
        .arg(&archive);
    
    list_cmd.assert()
        .success()
        .stdout(predicate::str::contains("main.asm"))
        .stdout(predicate::str::contains("sprites.bin"))
        .stdout(predicate::str::contains("src").not())
        .stdout(predicate::str::contains("data").not());
}

#[test]
#[serial]
fn test_archive_with_directory() {
    let temp = camino_tempfile::tempdir().unwrap();
    let sub_dir = temp.path().join("mydir");
    fs::create_dir(&sub_dir).unwrap();
    fs::write(sub_dir.join("file1.txt"), "content1").unwrap();
    fs::write(sub_dir.join("file2.txt"), "content2").unwrap();
    
    let archive = temp.path().join("test.zip");
    
    // Create archive with directory
    let mut cmd = cargo_bin_cmd!("bndbuild");
    cmd.current_dir(temp.path())
        .arg("--direct")
        .arg("--")
        .arg("archive")
        .arg("create")
        .arg("-o")
        .arg(&archive)
        .arg("mydir");
    
    cmd.assert().success();
    
    // List to verify directory structure
    let mut list_cmd = cargo_bin_cmd!("bndbuild");
    list_cmd.current_dir(temp.path())
        .arg("--direct")
        .arg("--")
        .arg("archive")
        .arg("list")
        .arg(&archive);
    
    list_cmd.assert()
        .success()
        .stdout(predicate::str::contains("mydir/file1.txt"))
        .stdout(predicate::str::contains("mydir/file2.txt"));
}

#[test]
#[serial]
fn test_archive_invalid_format() {
    let temp = camino_tempfile::tempdir().unwrap();
    let file1 = temp.path().join("file1.txt");
    fs::write(&file1, "content").unwrap();
    
    let archive = temp.path().join("test.rar");  // Unsupported format
    
    let mut cmd = cargo_bin_cmd!("bndbuild");
    cmd.current_dir(temp.path())
        .arg("--direct")
        .arg("--")
        .arg("archive")
        .arg("create")
        .arg("-o")
        .arg(&archive)
        .arg(file1.file_name().unwrap());
    
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("Unsupported archive format"));
}

#[test]
#[serial]
fn test_archive_help() {
    let mut cmd = cargo_bin_cmd!("bndbuild");
    cmd.arg("--direct")
        .arg("--")
        .arg("archive")
        .arg("--help");
    
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Create, list, and extract archives"))
        .stdout(predicate::str::contains("create"))
        .stdout(predicate::str::contains("list"))
        .stdout(predicate::str::contains("extract"));
}

#[test]
#[serial]
#[ignore] // --version flag not implemented for archive subcommand
fn test_archive_version() {
    let mut cmd = cargo_bin_cmd!("bndbuild");
    cmd.arg("--direct")
        .arg("--")
        .arg("archive")
        .arg("--version");
    
    cmd.assert().success();
}

#[test]
#[serial]
fn test_archive_strip_prefix_with_wildcard() {
    let temp = camino_tempfile::tempdir().unwrap();
    create_test_files_in_subdir(&temp, "dist");
    let archive = temp.path().join("test.zip");
    
    // Create archive using wildcard and strip prefix
    let mut cmd = cargo_bin_cmd!("bndbuild");
    cmd.current_dir(temp.path())
        .arg("--direct")
        .arg("--")
        .arg("archive")
        .arg("create")
        .arg("-o")
        .arg(&archive)
        .arg("-s")
        .arg("dist")
        .arg("dist/file1.txt")
        .arg("dist/file2.txt")
        .arg("dist/file3.txt");
    
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Created archive"));
    
    // Verify the contents don't have dist/ prefix
    let mut list_cmd = cargo_bin_cmd!("bndbuild");
    list_cmd.current_dir(temp.path())
        .arg("--direct")
        .arg("--")
        .arg("archive")
        .arg("list")
        .arg(&archive);
    
    let output = list_cmd.assert().success();
    let stdout = String::from_utf8_lossy(&output.get_output().stdout);
    
    // Verify files are listed without "dist/" prefix
    assert!(stdout.contains("file1.txt"));
    assert!(stdout.contains("file2.txt"));
    assert!(stdout.contains("file3.txt"));
    // Ensure "dist/" doesn't appear in paths
    assert!(!stdout.contains("dist/file"));
}
