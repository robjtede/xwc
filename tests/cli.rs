use assert_cmd::Command;

#[test]
fn default_output_is_left_aligned_with_headings() {
    let mut cmd = Command::cargo_bin("vwc").unwrap();

    cmd.write_stdin("one two\nthree\n")
        .assert()
        .success()
        .stdout("lines bytes\n2     14\n");
}
