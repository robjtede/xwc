use std::{
    fs::{self, File},
    io::{self, BufReader},
    num::NonZeroUsize,
    process::ExitCode,
    thread,
};

use bytesize::ByteSize;
use clap::Parser;
use glob::glob;
use rayon::prelude::*;
use xwc::{CountOptions, Counts, count_reader};

const BUFFER_SIZE: usize = 64 * 1024;
const DEFAULT_PARALLEL_FILE_THRESHOLD: usize = 3;

#[derive(Debug, Eq, PartialEq)]
struct Config {
    show_lines: bool,
    show_words: bool,
    show_bytes: bool,
    show_headings: bool,
    human_readable: bool,
    jobs: Option<usize>,
    globs: Vec<String>,
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

    #[arg(
        short = 'j',
        long = "jobs",
        value_name = "N",
        help = "Set the worker count for multiple input files"
    )]
    jobs: Option<NonZeroUsize>,

    #[arg(
        long = "glob",
        value_name = "PATTERN",
        help = "Add files matching PATTERN"
    )]
    globs: Vec<String>,

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
            jobs: self.jobs.map(NonZeroUsize::get),
            globs: self.globs,
            files: self.files,
        }
    }
}

fn run(config: &Config) -> bool {
    let count_options = CountOptions {
        lines: config.show_lines,
        words: config.show_words,
    };
    let paths = match input_paths(config) {
        Ok(paths) => paths,
        Err(error) => {
            eprintln!("xwc: --glob: {error}");
            return false;
        }
    };

    if paths.is_empty() {
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

    for file_count in count_paths(&paths, count_options, config.jobs) {
        match file_count.result {
            Ok(counts) => {
                total += counts;
                rows.push((counts, Some(file_count.path)));
            }
            Err(error) => {
                had_error = true;
                eprintln!("xwc: {}: {error}", file_count.path);
            }
        }
    }

    if paths.len() > 1 {
        rows.push((total, Some("total")));
    }

    print_rows(config, rows);

    !had_error
}

fn input_paths(config: &Config) -> Result<Vec<String>, String> {
    let mut paths = config.files.clone();

    for pattern in &config.globs {
        let entries = glob(pattern).map_err(|error| format!("{pattern}: {error}"))?;
        let mut matches = Vec::new();

        for entry in entries {
            match entry {
                Ok(path) => matches.push(path.to_string_lossy().into_owned()),
                Err(error) => return Err(format!("{}: {error}", error.path().display())),
            }
        }

        if matches.is_empty() {
            return Err(format!("{pattern}: no matches"));
        }

        matches.sort();
        paths.extend(matches);
    }

    Ok(paths)
}

#[derive(Debug)]
struct FileCount<'a> {
    path: &'a str,
    result: io::Result<Counts>,
}

fn count_paths<'a>(
    paths: &'a [String],
    options: CountOptions,
    jobs: Option<usize>,
) -> Vec<FileCount<'a>> {
    let parallelism = worker_count(paths, jobs);

    let Some(parallelism) = parallelism else {
        return paths
            .iter()
            .map(|path| FileCount {
                path,
                result: count_path(path, options),
            })
            .collect();
    };

    rayon::ThreadPoolBuilder::new()
        .num_threads(parallelism)
        .build()
        .expect("parallelism must be non-zero")
        .install(|| {
            paths
                .par_iter()
                .map(|path| FileCount {
                    path,
                    result: count_path(path, options),
                })
                .collect()
        })
}

fn worker_count(paths: &[String], jobs: Option<usize>) -> Option<usize> {
    if paths.len() <= 1 || paths.iter().any(|path| path == "-") || jobs == Some(1) {
        return None;
    }

    if jobs.is_none() && paths.len() <= DEFAULT_PARALLEL_FILE_THRESHOLD {
        return None;
    }

    Some(
        jobs.or_else(|| thread::available_parallelism().map(usize::from).ok())
            .unwrap_or(1)
            .min(paths.len()),
    )
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
                jobs: None,
                globs: Vec::new(),
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
                jobs: None,
                globs: Vec::new(),
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
                jobs: None,
                globs: Vec::new(),
                files: Vec::new()
            }
        );
    }

    #[test]
    fn parses_jobs_option() {
        let config = Cli::try_parse_from(["xwc", "-j", "3", "a", "b"])
            .unwrap()
            .into_config();

        assert_eq!(config.jobs, Some(3));
    }

    #[test]
    fn parses_glob_options() {
        let config = Cli::try_parse_from(["xwc", "--glob", "src/*.rs", "--glob", "tests/*.rs"])
            .unwrap()
            .into_config();

        assert_eq!(
            config.globs,
            vec!["src/*.rs".to_owned(), "tests/*.rs".to_owned()]
        );
    }

    #[test]
    fn rejects_zero_jobs() {
        assert!(Cli::try_parse_from(["xwc", "-j", "0"]).is_err());
    }

    #[test]
    fn defaults_to_serial_until_parallel_file_threshold() {
        let paths = vec!["a".to_owned(), "b".to_owned(), "c".to_owned()];

        assert_eq!(worker_count(&paths, None), None);
    }

    #[test]
    fn defaults_to_parallel_after_parallel_file_threshold() {
        let paths = vec![
            "a".to_owned(),
            "b".to_owned(),
            "c".to_owned(),
            "d".to_owned(),
        ];

        assert!(worker_count(&paths, None).is_some());
    }

    #[test]
    fn explicit_jobs_overrides_parallel_file_threshold() {
        let paths = vec!["a".to_owned(), "b".to_owned()];

        assert_eq!(worker_count(&paths, Some(2)), Some(2));
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
    fn input_paths_expands_globs_in_sorted_order() {
        let directory = tempfile::tempdir().unwrap();
        let path_b = directory.path().join("b.txt");
        let path_a = directory.path().join("a.txt");
        fs::write(&path_b, "one\n").unwrap();
        fs::write(&path_a, "two\n").unwrap();
        let pattern = directory
            .path()
            .join("*.txt")
            .to_string_lossy()
            .into_owned();
        let config = Config {
            show_lines: true,
            show_words: false,
            show_bytes: true,
            show_headings: true,
            human_readable: false,
            jobs: None,
            globs: vec![pattern],
            files: vec!["literal.txt".to_owned()],
        };

        assert_eq!(
            input_paths(&config).unwrap(),
            vec![
                "literal.txt".to_owned(),
                path_a.to_string_lossy().into_owned(),
                path_b.to_string_lossy().into_owned(),
            ]
        );
    }

    #[test]
    fn input_paths_rejects_globs_without_matches() {
        let config = Config {
            show_lines: true,
            show_words: false,
            show_bytes: true,
            show_headings: true,
            human_readable: false,
            jobs: None,
            globs: vec!["missing-*".to_owned()],
            files: Vec::new(),
        };

        assert_eq!(
            input_paths(&config).unwrap_err(),
            "missing-*: no matches".to_owned()
        );
    }

    #[test]
    fn counts_multiple_paths_in_input_order() {
        let first = NamedTempFile::new().unwrap();
        let second = NamedTempFile::new().unwrap();
        fs::write(first.path(), "one\ntwo\n").unwrap();
        fs::write(second.path(), "three four\n").unwrap();
        let paths = vec![
            first.path().to_string_lossy().into_owned(),
            second.path().to_string_lossy().into_owned(),
        ];

        let file_counts = count_paths(
            &paths,
            CountOptions {
                lines: true,
                words: true,
            },
            Some(2),
        );

        assert_eq!(
            file_counts
                .iter()
                .map(|file_count| file_count.path)
                .collect::<Vec<_>>(),
            paths.iter().map(String::as_str).collect::<Vec<_>>()
        );
        assert_eq!(
            file_counts
                .into_iter()
                .map(|file_count| file_count.result.unwrap())
                .collect::<Vec<_>>(),
            vec![
                Counts {
                    lines: 2,
                    words: 2,
                    bytes: 8,
                },
                Counts {
                    lines: 1,
                    words: 2,
                    bytes: 11,
                },
            ]
        );
    }

    #[test]
    fn formats_human_readable_bytes() {
        assert_eq!(format_byte_count(1024, true), "1.0K");
        assert_eq!(format_byte_count(1024, false), "1024");
    }
}
