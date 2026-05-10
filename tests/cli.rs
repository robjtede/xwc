use std::fs;

use assert_cmd::Command;

#[test]
fn default_output_is_left_aligned_with_headings() {
    let mut cmd = Command::cargo_bin("xwc").unwrap();

    cmd.write_stdin("one two\nthree\n")
        .assert()
        .success()
        .stdout("lines  bytes\n2      14\n");
}

#[test]
fn include_words_adds_a_left_aligned_word_column() {
    let mut cmd = Command::cargo_bin("xwc").unwrap();

    cmd.arg("--include-words")
        .write_stdin("one two\nthree\n")
        .assert()
        .success()
        .stdout("lines  words  bytes\n2      3      14\n");
}

#[test]
fn include_chars_adds_a_left_aligned_char_column() {
    let mut cmd = Command::cargo_bin("xwc").unwrap();

    cmd.arg("--include-chars")
        .write_stdin("café\n")
        .assert()
        .success()
        .stdout("lines  chars  bytes\n1      5      6\n");
}

#[test]
fn include_longest_line_adds_a_left_aligned_longest_line_column() {
    let mut cmd = Command::cargo_bin("xwc").unwrap();

    cmd.arg("--include-longest-line")
        .write_stdin("one\nthree\ncafé\n")
        .assert()
        .success()
        .stdout("lines  max-line  bytes\n3      5         16\n");
}

#[test]
fn words_option_counts_words() {
    let mut cmd = Command::cargo_bin("xwc").unwrap();

    cmd.arg("--words")
        .write_stdin("one two\nthree\n")
        .assert()
        .success()
        .stdout("3\n");
}

#[test]
fn chars_option_counts_utf8_characters() {
    let mut cmd = Command::cargo_bin("xwc").unwrap();

    cmd.arg("--chars")
        .write_stdin("café\n")
        .assert()
        .success()
        .stdout("5\n");
}

#[test]
fn longest_line_option_counts_longest_line() {
    let mut cmd = Command::cargo_bin("xwc").unwrap();

    cmd.arg("--longest-line")
        .write_stdin("one\nthree\ncafé\n")
        .assert()
        .success()
        .stdout("5\n");
}

#[test]
fn chars_and_bytes_can_be_counted_together() {
    let mut cmd = Command::cargo_bin("xwc").unwrap();

    cmd.arg("-mc")
        .write_stdin("café\n")
        .assert()
        .success()
        .stdout("chars  bytes\n5      6\n");
}

#[test]
fn all_option_counts_everything() {
    let mut cmd = Command::cargo_bin("xwc").unwrap();

    cmd.arg("--all")
        .write_stdin("café\n")
        .assert()
        .success()
        .stdout("lines  words  chars  max-line  bytes\n1      1      5      4         6\n");
}

#[test]
fn human_readable_default_output_uses_size_heading() {
    let mut cmd = Command::cargo_bin("xwc").unwrap();

    cmd.arg("-h")
        .write_stdin("one two\nthree\n")
        .assert()
        .success()
        .stdout("lines  size\n2      14B\n");
}

#[test]
fn self_profile_adds_a_duration_column() {
    let mut cmd = Command::cargo_bin("xwc").unwrap();

    let assert = cmd
        .arg("--self-profile")
        .write_stdin("one two\nthree\n")
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let lines = stdout.lines().collect::<Vec<_>>();
    let fields = lines[1].split_whitespace().collect::<Vec<_>>();

    assert_eq!(lines[0], "lines  bytes  duration");
    assert_eq!(fields[0], "2");
    assert_eq!(fields[1], "14");
    assert!(
        fields[2].ends_with("ns")
            || fields[2].ends_with("us")
            || fields[2].ends_with("ms")
            || fields[2].ends_with('s')
    );
}

#[test]
fn self_profile_places_duration_after_file_column() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("a.txt");
    fs::write(&path, "one\n").unwrap();
    let path = path.to_string_lossy();
    let mut cmd = Command::cargo_bin("xwc").unwrap();

    let assert = cmd
        .arg("--self-profile")
        .arg(path.as_ref())
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let lines = stdout.lines().collect::<Vec<_>>();
    let headings = lines[0].split_whitespace().collect::<Vec<_>>();
    let fields = lines[1].split_whitespace().collect::<Vec<_>>();

    assert_eq!(headings, vec!["lines", "bytes", "file", "duration"]);
    assert_eq!(fields[0], "1");
    assert_eq!(fields[1], "4");
    assert_eq!(fields[2], path);
    assert!(
        fields[3].ends_with("ns")
            || fields[3].ends_with("us")
            || fields[3].ends_with("ms")
            || fields[3].ends_with('s')
    );
}

#[test]
fn glob_option_counts_matching_files() {
    let directory = tempfile::tempdir().unwrap();
    let path_b = directory.path().join("b.txt");
    let path_a = directory.path().join("a.txt");
    fs::write(&path_b, "one\n").unwrap();
    fs::write(&path_a, "two\nthree\n").unwrap();
    let pattern = directory
        .path()
        .join("*.txt")
        .to_string_lossy()
        .into_owned();
    let path_a = path_a.to_string_lossy();
    let path_b = path_b.to_string_lossy();
    let expected = format!(
        "lines  bytes  file\n2      10     {path_a}\n1      4      {path_b}\n3      14     total\n"
    );
    let mut cmd = Command::cargo_bin("xwc").unwrap();

    cmd.arg("--glob")
        .arg(pattern)
        .assert()
        .success()
        .stdout(expected);
}

#[test]
fn sort_by_sorts_file_rows_and_keeps_total_last() {
    let directory = tempfile::tempdir().unwrap();
    let path_b = directory.path().join("b.txt");
    let path_a = directory.path().join("a.txt");
    fs::write(&path_b, "one\n").unwrap();
    fs::write(&path_a, "two\nthree\n").unwrap();
    let pattern = directory
        .path()
        .join("*.txt")
        .to_string_lossy()
        .into_owned();
    let path_a = path_a.to_string_lossy();
    let path_b = path_b.to_string_lossy();
    let expected = format!(
        "lines  bytes  file\n1      4      {path_b}\n2      10     {path_a}\n3      14     total\n"
    );
    let mut cmd = Command::cargo_bin("xwc").unwrap();

    cmd.arg("--glob")
        .arg(pattern)
        .arg("--sort-by")
        .arg("bytes")
        .assert()
        .success()
        .stdout(expected);
}
