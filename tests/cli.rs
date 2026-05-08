use assert_cmd::Command;

#[test]
fn default_output_is_left_aligned_with_headings() {
    let mut cmd = Command::cargo_bin("vwc").unwrap();

    cmd.write_stdin("one two\nthree\n")
        .assert()
        .success()
        .stdout("lines  size\n2      14\n");
}

#[test]
fn include_words_adds_a_left_aligned_word_column() {
    let mut cmd = Command::cargo_bin("vwc").unwrap();

    cmd.arg("--include-words")
        .write_stdin("one two\nthree\n")
        .assert()
        .success()
        .stdout("lines  words  size\n2      3      14\n");
}
