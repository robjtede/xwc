use std::{
    fs::{self, File},
    io::{self, BufReader},
    process::ExitCode,
};

use bytesize::ByteSize;
use clap::Parser;
use xwc::{CountOptions, Counts, count_reader};

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
    name = "xwc",
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
    let count_options = CountOptions {
        lines: config.show_lines,
        words: config.show_words,
    };

    if config.files.is_empty() {
        let stdin = io::stdin();
        match count_reader(stdin.lock(), count_options) {
            Ok(counts) => {
                print_rows(config, vec![(counts, None)]);
                return true;
            }
            Err(error) => {
                eprintln!("xwc: {error}");
                return false;
            }
        }
    }

    let mut total = Counts::default();
    let mut had_error = false;
    let mut rows = Vec::new();

    for path in &config.files {
        match count_path(path, count_options) {
            Ok(counts) => {
                total += counts;
                rows.push((counts, Some(path.as_str())));
            }
            Err(error) => {
                had_error = true;
                eprintln!("xwc: {path}: {error}");
            }
        }
    }

    if config.files.len() > 1 {
        rows.push((total, Some("total")));
    }

    print_rows(config, rows);

    !had_error
}

fn count_path(path: &str, options: CountOptions) -> io::Result<Counts> {
    if path == "-" {
        let stdin = io::stdin();
        return count_reader(stdin.lock(), options);
    }

    if !options.lines && !options.words {
        let metadata = fs::metadata(path)?;

        if metadata.is_file() {
            return Ok(Counts {
                bytes: metadata.len(),
                ..Counts::default()
            });
        }
    }

    let file = File::open(path)?;
    count_reader(BufReader::with_capacity(BUFFER_SIZE, file), options)
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
        fields.push(byte_heading(config).to_owned());
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

fn byte_heading(config: &Config) -> &'static str {
    if config.human_readable {
        "size"
    } else {
        "bytes"
    }
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

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::NamedTempFile;

    use super::*;

    #[test]
    fn default_config_counts_lines_and_bytes() {
        let config = Cli::try_parse_from(["xwc"]).unwrap().into_config();

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
        let config = Cli::try_parse_from(["xwc", "-lc", "--human-readable", "a", "b"])
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
        let config = Cli::try_parse_from(["xwc", "-W"]).unwrap().into_config();

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
    fn byte_only_path_count_does_not_count_lines_or_words() {
        let file = NamedTempFile::new().unwrap();
        fs::write(file.path(), "one\ntwo\nthree\n").unwrap();

        let counts = count_path(
            file.path().to_str().unwrap(),
            CountOptions {
                lines: false,
                words: false,
            },
        )
        .unwrap();

        assert_eq!(
            counts,
            Counts {
                lines: 0,
                words: 0,
                bytes: 14
            }
        );
    }

    #[test]
    fn formats_human_readable_bytes() {
        assert_eq!(format_byte_count(1024, true), "1.0K");
        assert_eq!(format_byte_count(1024, false), "1024");
    }
}
