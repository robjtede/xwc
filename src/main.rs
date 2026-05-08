use std::fs::File;
use std::io::{self, BufReader, Read};
use std::process::ExitCode;

use bytesize::ByteSize;
use clap::Parser;

const BUFFER_SIZE: usize = 64 * 1024;

#[derive(Debug, Eq, PartialEq)]
struct Config {
    show_lines: bool,
    show_words: bool,
    show_bytes: bool,
    show_headings: bool,
    human_readable: bool,
    files: Vec<String>,
}

#[derive(Debug, Parser)]
#[command(
    name = "vwc",
    about = "Count lines and bytes for each FILE, or standard input when no FILE is given.",
    disable_help_flag = true
)]
struct Cli {
    #[arg(short = 'l', long = "lines", help = "Print the newline count")]
    lines: bool,

    #[arg(short = 'w', long = "words", help = "Print the word count")]
    words: bool,

    #[arg(short = 'W', long = "include-words", help = "Include the word count")]
    include_words: bool,

    #[arg(short = 'c', long = "bytes", help = "Print the byte count")]
    bytes: bool,

    #[arg(
        short = 'h',
        long = "human-readable",
        help = "Print byte counts in human-readable IEC units"
    )]
    human_readable: bool,

    #[arg(long = "help", action = clap::ArgAction::Help, help = "Print help")]
    help: Option<bool>,

    #[arg(value_name = "FILE")]
    files: Vec<String>,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct Counts {
    lines: u64,
    words: u64,
    bytes: u64,
}

#[derive(Debug, Default)]
struct WordState {
    in_word: bool,
    pending_utf8: Vec<u8>,
}

fn main() -> ExitCode {
    let config = Cli::parse().into_config();

    if run(&config) {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

impl Cli {
    fn into_config(self) -> Config {
        let has_count_option = self.lines || self.words || self.bytes;

        Config {
            show_lines: self.lines || !has_count_option,
            show_words: self.words || self.include_words,
            show_bytes: self.bytes || !has_count_option,
            show_headings: !has_count_option,
            human_readable: self.human_readable,
            files: self.files,
        }
    }
}

fn run(config: &Config) -> bool {
    if config.files.is_empty() {
        let stdin = io::stdin();
        match count_reader(stdin.lock()) {
            Ok(counts) => {
                print_rows(config, vec![(counts, None)]);
                return true;
            }
            Err(error) => {
                eprintln!("vwc: {error}");
                return false;
            }
        }
    }

    let mut total = Counts::default();
    let mut had_error = false;
    let mut rows = Vec::new();

    for path in &config.files {
        match count_path(path) {
            Ok(counts) => {
                total += counts;
                rows.push((counts, Some(path.as_str())));
            }
            Err(error) => {
                had_error = true;
                eprintln!("vwc: {path}: {error}");
            }
        }
    }

    if config.files.len() > 1 {
        rows.push((total, Some("total")));
    }

    print_rows(config, rows);

    !had_error
}

fn count_path(path: &str) -> io::Result<Counts> {
    if path == "-" {
        let stdin = io::stdin();
        return count_reader(stdin.lock());
    }

    let file = File::open(path)?;
    count_reader(BufReader::with_capacity(BUFFER_SIZE, file))
}

fn count_reader(mut reader: impl Read) -> io::Result<Counts> {
    let mut counts = Counts::default();
    let mut buffer = [0; BUFFER_SIZE];
    let mut word_state = WordState::default();

    loop {
        let read = reader.read(&mut buffer)?;
        if read == 0 {
            break;
        }

        counts.bytes += read as u64;
        counts.lines += bytecount_newlines(&buffer[..read]) as u64;
        counts.words += count_words(&buffer[..read], &mut word_state) as u64;
    }

    if !word_state.pending_utf8.is_empty() && !word_state.in_word {
        counts.words += 1;
    }

    Ok(counts)
}

fn bytecount_newlines(buffer: &[u8]) -> usize {
    buffer.iter().filter(|&&byte| byte == b'\n').count()
}

fn count_words(buffer: &[u8], state: &mut WordState) -> usize {
    let mut words = 0;
    let combined;

    let buffer = if state.pending_utf8.is_empty() {
        buffer
    } else {
        combined = {
            let mut bytes = std::mem::take(&mut state.pending_utf8);
            bytes.extend_from_slice(buffer);
            bytes
        };
        &combined
    };

    let mut offset = 0;

    while offset < buffer.len() {
        match std::str::from_utf8(&buffer[offset..]) {
            Ok(valid) => {
                words += count_words_in_str(valid, &mut state.in_word);
                break;
            }
            Err(error) => {
                let valid_end = offset + error.valid_up_to();
                let valid = std::str::from_utf8(&buffer[offset..valid_end])
                    .expect("valid_up_to must split at a UTF-8 boundary");
                words += count_words_in_str(valid, &mut state.in_word);
                offset = valid_end;

                if let Some(error_len) = error.error_len() {
                    if !state.in_word {
                        words += 1;
                        state.in_word = true;
                    }
                    offset += error_len;
                } else {
                    state.pending_utf8.extend_from_slice(&buffer[offset..]);
                    break;
                }
            }
        }
    }

    words
}

fn count_words_in_str(input: &str, in_word: &mut bool) -> usize {
    let mut words = 0;

    for ch in input.chars() {
        if ch.is_whitespace() {
            *in_word = false;
        } else if !*in_word {
            words += 1;
            *in_word = true;
        }
    }

    words
}

fn print_rows(config: &Config, rows: Vec<(Counts, Option<&str>)>) {
    let has_labels = rows.iter().any(|(_, label)| label.is_some());
    let mut rendered_rows = Vec::new();

    if config.show_headings {
        rendered_rows.push(headings(config, has_labels));
    }

    for (counts, label) in rows {
        rendered_rows.push(fields(config, counts, label));
    }

    let widths = column_widths(&rendered_rows);

    for row in rendered_rows {
        print_row(&row, &widths);
    }
}

fn headings(config: &Config, has_labels: bool) -> Vec<String> {
    let mut fields = Vec::new();

    if config.show_lines {
        fields.push("lines".to_owned());
    }

    if config.show_words {
        fields.push("words".to_owned());
    }

    if config.show_bytes {
        fields.push("size".to_owned());
    }

    if has_labels {
        fields.push("file".to_owned());
    }

    fields
}

fn fields(config: &Config, counts: Counts, label: Option<&str>) -> Vec<String> {
    let mut fields = Vec::new();

    if config.show_lines {
        fields.push(counts.lines.to_string());
    }

    if config.show_words {
        fields.push(counts.words.to_string());
    }

    if config.show_bytes {
        fields.push(format_byte_count(counts.bytes, config.human_readable));
    }

    if let Some(label) = label {
        fields.push(label.to_owned());
    }

    fields
}

fn column_widths(rows: &[Vec<String>]) -> Vec<usize> {
    let column_count = rows.iter().map(Vec::len).max().unwrap_or(0);
    let mut widths = vec![0; column_count];

    for row in rows {
        for (index, field) in row.iter().enumerate() {
            widths[index] = widths[index].max(field.len());
        }
    }

    widths
}

fn print_row(row: &[String], widths: &[usize]) {
    for (index, field) in row.iter().enumerate() {
        if index > 0 {
            print!("  ");
        }

        if index + 1 == row.len() {
            print!("{field}");
        } else {
            print!("{field:<width$}", width = widths[index]);
        }
    }

    println!();
}

fn format_byte_count(bytes: u64, human_readable: bool) -> String {
    if human_readable {
        ByteSize::b(bytes).display().iec_short().to_string()
    } else {
        bytes.to_string()
    }
}

impl std::ops::AddAssign for Counts {
    fn add_assign(&mut self, rhs: Self) {
        self.lines += rhs.lines;
        self.words += rhs.words;
        self.bytes += rhs.bytes;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_counts_lines_and_bytes() {
        let config = Cli::try_parse_from(["vwc"]).unwrap().into_config();

        assert_eq!(
            config,
            Config {
                show_lines: true,
                show_words: false,
                show_bytes: true,
                show_headings: true,
                human_readable: false,
                files: Vec::new()
            }
        );
    }

    #[test]
    fn parses_combined_short_options_and_files() {
        let config = Cli::try_parse_from(["vwc", "-lc", "--human-readable", "a", "b"])
            .unwrap()
            .into_config();

        assert_eq!(
            config,
            Config {
                show_lines: true,
                show_words: false,
                show_bytes: true,
                show_headings: false,
                human_readable: true,
                files: vec!["a".to_owned(), "b".to_owned()]
            }
        );
    }

    #[test]
    fn include_words_adds_words_to_default_columns() {
        let config = Cli::try_parse_from(["vwc", "-W"]).unwrap().into_config();

        assert_eq!(
            config,
            Config {
                show_lines: true,
                show_words: true,
                show_bytes: true,
                show_headings: true,
                human_readable: false,
                files: Vec::new()
            }
        );
    }

    #[test]
    fn counts_newlines_and_bytes_without_decoding_utf8() {
        let input = "cafe\ncafé\n東京 京都".as_bytes();

        assert_eq!(
            count_reader(input).unwrap(),
            Counts {
                lines: 2,
                words: 4,
                bytes: 24
            }
        );
    }

    #[test]
    fn counts_words_across_buffer_boundaries() {
        let mut state = WordState::default();

        assert_eq!(count_words(b"hello", &mut state), 1);
        assert_eq!(count_words(b"world\nagain", &mut state), 1);
        assert_eq!(count_words(b" later", &mut state), 1);
    }

    #[test]
    fn counts_utf8_words_across_buffer_boundaries() {
        let mut state = WordState::default();
        let input = "東京 京都".as_bytes();

        assert_eq!(count_words(&input[..4], &mut state), 1);
        assert_eq!(count_words(&input[4..], &mut state), 1);
    }

    #[test]
    fn formats_human_readable_bytes() {
        assert_eq!(format_byte_count(1024, true), "1.0K");
        assert_eq!(format_byte_count(1024, false), "1024");
    }
}
