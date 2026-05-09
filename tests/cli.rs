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
fn words_option_counts_words() {
    let mut cmd = Command::cargo_bin("xwc").unwrap();

    cmd.arg("--words")
        .write_stdin("one two\nthree\n")
        .assert()
        .success()
        .stdout("3\n");
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
