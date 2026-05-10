use std::{
    fs::{self, File},
    io::{self, BufReader},
    num::NonZeroUsize,
    process::ExitCode,
    time::{Duration, Instant},
};

use clap::{Parser, ValueEnum};
use glob::glob;
use rayon::prelude::*;
use xwc::{
    Config, CountOptions, Counts, OutputRow, SortBy, SortOrder, column_widths, count_reader,
    render_rows, worker_count,
};

const BUFFER_SIZE: usize = 64 * 1024;

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

    #[arg(short = 'm', long = "chars", help = "Print the character count")]
    chars: bool,

    #[arg(
        short = 'M',
        long = "include-chars",
        help = "Include the character count"
    )]
    include_chars: bool,

    #[arg(short = 'W', long = "include-words", help = "Include the word count")]
    include_words: bool,

    #[arg(long = "longest-line", help = "Print the longest line length")]
    longest_line: bool,

    #[arg(
        long = "include-longest-line",
        help = "Include the longest line length"
    )]
    include_longest_line: bool,

    #[arg(short = 'c', long = "bytes", help = "Print the byte count")]
    bytes: bool,

    #[arg(short = 'A', long = "all", help = "Print all counts")]
    all: bool,

    #[arg(
        short = 'h',
        long = "human-readable",
        help = "Print byte counts in human-readable IEC units"
    )]
    human_readable: bool,

    #[arg(
        long = "self-profile",
        help = "Include per-input counting duration in the output"
    )]
    self_profile: bool,

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

    #[arg(
        long = "sort-by",
        value_enum,
        value_name = "COLUMN",
        help = "Sort output rows by COLUMN"
    )]
    sort_by: Option<CliSortBy>,

    #[arg(
        long = "sort-order",
        value_enum,
        default_value_t = CliSortOrder::Asc,
        value_name = "ORDER",
        help = "Set output sort order"
    )]
    sort_order: CliSortOrder,

    #[arg(long = "help", action = clap::ArgAction::Help, help = "Print help")]
    help: Option<bool>,

    #[arg(value_name = "FILE")]
    files: Vec<String>,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum CliSortBy {
    Lines,
    Words,
    Chars,
    #[value(name = "max-line")]
    MaxLine,
    Bytes,
    File,
    Duration,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum CliSortOrder {
    Asc,
    Desc,
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
        let has_count_option =
            self.lines || self.words || self.chars || self.bytes || self.longest_line || self.all;
        let show_lines = self.lines || self.all || !has_count_option;
        let show_words = self.words || self.all || self.include_words;
        let show_chars = self.chars || self.all || self.include_chars;
        let show_bytes = self.bytes || self.all || !has_count_option;
        let show_max_line_length = self.longest_line || self.include_longest_line || self.all;
        let shown_metric_count = show_lines as u8
            + show_words as u8
            + show_chars as u8
            + show_bytes as u8
            + show_max_line_length as u8
            + self.self_profile as u8;

        Config {
            show_lines,
            show_words,
            show_chars,
            show_bytes,
            show_max_line_length,
            show_headings: shown_metric_count > 1,
            human_readable: self.human_readable,
            self_profile: self.self_profile,
            jobs: self.jobs.map(NonZeroUsize::get),
            sort_by: self.sort_by.map(Into::into),
            sort_order: self.sort_order.into(),
            globs: self.globs,
            files: self.files,
        }
    }
}

impl From<CliSortBy> for SortBy {
    fn from(sort_by: CliSortBy) -> Self {
        match sort_by {
            CliSortBy::Lines => Self::Lines,
            CliSortBy::Words => Self::Words,
            CliSortBy::Chars => Self::Chars,
            CliSortBy::MaxLine => Self::MaxLine,
            CliSortBy::Bytes => Self::Bytes,
            CliSortBy::File => Self::File,
            CliSortBy::Duration => Self::Duration,
        }
    }
}

impl From<CliSortOrder> for SortOrder {
    fn from(sort_order: CliSortOrder) -> Self {
        match sort_order {
            CliSortOrder::Asc => Self::Asc,
            CliSortOrder::Desc => Self::Desc,
        }
    }
}

fn run(config: &Config) -> bool {
    let count_options = config.count_options();
    let paths = match input_paths(config) {
        Ok(paths) => paths,
        Err(error) => {
            eprintln!("xwc: --glob: {error}");
            return false;
        }
    };

    if paths.is_empty() {
        let stdin = io::stdin();
        let started = Instant::now();
        match count_reader(stdin.lock(), count_options) {
            Ok(counts) => {
                let duration = config.self_profile.then(|| started.elapsed());
                print_rows(
                    config,
                    vec![OutputRow {
                        counts,
                        duration,
                        label: None,
                    }],
                );
                return true;
            }
            Err(error) => {
                eprintln!("xwc: {error}");
                return false;
            }
        }
    }

    let mut total = Counts::default();
    let mut total_duration = Duration::default();
    let mut had_error = false;
    let mut rows = Vec::new();

    for file_count in count_paths(&paths, count_options, config.jobs, config.self_profile) {
        match file_count.result {
            Ok(profiled_counts) => {
                let counts = profiled_counts.counts;
                total += counts;
                if let Some(duration) = profiled_counts.duration {
                    total_duration += duration;
                }
                rows.push(OutputRow {
                    counts,
                    duration: profiled_counts.duration,
                    label: Some(file_count.path),
                });
            }
            Err(error) => {
                had_error = true;
                eprintln!("xwc: {}: {error}", file_count.path);
            }
        }
    }

    sort_rows(config, &mut rows);

    if paths.len() > 1 {
        rows.push(OutputRow {
            counts: total,
            duration: config.self_profile.then_some(total_duration),
            label: Some("total"),
        });
    }

    print_rows(config, rows);

    !had_error
}

fn sort_rows(config: &Config, rows: &mut [OutputRow<'_>]) {
    let Some(sort_by) = config.sort_by else {
        return;
    };

    rows.sort_by(|left, right| {
        let ordering = match sort_by {
            SortBy::Lines => left.counts.lines.cmp(&right.counts.lines),
            SortBy::Words => left.counts.words.cmp(&right.counts.words),
            SortBy::Chars => left.counts.chars.cmp(&right.counts.chars),
            SortBy::MaxLine => left
                .counts
                .max_line_length
                .cmp(&right.counts.max_line_length),
            SortBy::Bytes => left.counts.bytes.cmp(&right.counts.bytes),
            SortBy::File => left
                .label
                .unwrap_or_default()
                .cmp(right.label.unwrap_or_default()),
            SortBy::Duration => left.duration.cmp(&right.duration),
        };

        match config.sort_order {
            SortOrder::Asc => ordering,
            SortOrder::Desc => ordering.reverse(),
        }
    });
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
    result: io::Result<ProfiledCounts>,
}

#[derive(Clone, Copy, Debug)]
struct ProfiledCounts {
    counts: Counts,
    duration: Option<Duration>,
}

fn count_paths<'a>(
    paths: &'a [String],
    options: CountOptions,
    jobs: Option<usize>,
    self_profile: bool,
) -> Vec<FileCount<'a>> {
    let parallelism = worker_count(paths, jobs);

    let Some(parallelism) = parallelism else {
        return paths
            .iter()
            .map(|path| FileCount {
                path,
                result: count_path_profiled(path, options, self_profile),
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
                    result: count_path_profiled(path, options, self_profile),
                })
                .collect()
        })
}

fn count_path_profiled(
    path: &str,
    options: CountOptions,
    self_profile: bool,
) -> io::Result<ProfiledCounts> {
    let started = Instant::now();
    let counts = count_path(path, options)?;

    Ok(ProfiledCounts {
        counts,
        duration: self_profile.then(|| started.elapsed()),
    })
}

fn count_path(path: &str, options: CountOptions) -> io::Result<Counts> {
    if path == "-" {
        let stdin = io::stdin();
        return count_reader(stdin.lock(), options);
    }

    if !options.lines && !options.words && !options.chars && !options.max_line_length {
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

fn print_rows(config: &Config, rows: Vec<OutputRow<'_>>) {
    let rendered_rows = render_rows(config, rows);
    let widths = column_widths(&rendered_rows);

    for row in rendered_rows {
        print_row(&row, &widths);
    }
}

fn print_row(row: &[String], widths: &[usize]) {
    for (index, (field, width)) in row.iter().zip(widths).enumerate() {
        if index > 0 {
            print!("  ");
        }

        if index + 1 == row.len() {
            print!("{field}");
        } else {
            print!("{field:<width$}");
        }
    }

    println!();
}

#[cfg(test)]
mod tests {
    use std::{fs, time::Duration};

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
                show_chars: false,
                show_bytes: true,
                show_max_line_length: false,
                show_headings: true,
                human_readable: false,
                self_profile: false,
                jobs: None,
                sort_by: None,
                sort_order: SortOrder::Asc,
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
                show_chars: false,
                show_bytes: true,
                show_max_line_length: false,
                show_headings: true,
                human_readable: true,
                self_profile: false,
                jobs: None,
                sort_by: None,
                sort_order: SortOrder::Asc,
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
                show_chars: false,
                show_bytes: true,
                show_max_line_length: false,
                show_headings: true,
                human_readable: false,
                self_profile: false,
                jobs: None,
                sort_by: None,
                sort_order: SortOrder::Asc,
                globs: Vec::new(),
                files: Vec::new()
            }
        );
    }

    #[test]
    fn include_chars_adds_chars_to_default_columns() {
        let config = Cli::try_parse_from(["xwc", "-M"]).unwrap().into_config();

        assert_eq!(
            config,
            Config {
                show_lines: true,
                show_words: false,
                show_chars: true,
                show_bytes: true,
                show_max_line_length: false,
                show_headings: true,
                human_readable: false,
                self_profile: false,
                jobs: None,
                sort_by: None,
                sort_order: SortOrder::Asc,
                globs: Vec::new(),
                files: Vec::new()
            }
        );
    }

    #[test]
    fn include_longest_line_adds_longest_line_to_default_columns() {
        let config = Cli::try_parse_from(["xwc", "--include-longest-line"])
            .unwrap()
            .into_config();

        assert_eq!(
            config,
            Config {
                show_lines: true,
                show_words: false,
                show_chars: false,
                show_bytes: true,
                show_max_line_length: true,
                show_headings: true,
                human_readable: false,
                self_profile: false,
                jobs: None,
                sort_by: None,
                sort_order: SortOrder::Asc,
                globs: Vec::new(),
                files: Vec::new()
            }
        );
    }

    #[test]
    fn all_selects_every_count_with_default_headings() {
        let config = Cli::try_parse_from(["xwc", "--all"]).unwrap().into_config();

        assert_eq!(
            config,
            Config {
                show_lines: true,
                show_words: true,
                show_chars: true,
                show_bytes: true,
                show_max_line_length: true,
                show_headings: true,
                human_readable: false,
                self_profile: false,
                jobs: None,
                sort_by: None,
                sort_order: SortOrder::Asc,
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
    fn parses_sort_options() {
        let config = Cli::try_parse_from(["xwc", "--sort-by", "bytes", "--sort-order", "desc"])
            .unwrap()
            .into_config();

        assert_eq!(config.sort_by, Some(SortBy::Bytes));
        assert_eq!(config.sort_order, SortOrder::Desc);
    }

    #[test]
    fn parses_duration_sort_option() {
        let config = Cli::try_parse_from(["xwc", "--sort-by", "duration"])
            .unwrap()
            .into_config();

        assert_eq!(config.sort_by, Some(SortBy::Duration));
    }

    #[test]
    fn self_profile_adds_duration_to_headed_output() {
        let config = Cli::try_parse_from(["xwc", "--longest-line", "--self-profile"])
            .unwrap()
            .into_config();

        assert!(config.self_profile);
        assert!(config.show_headings);
    }

    #[test]
    fn rejects_zero_jobs() {
        Cli::try_parse_from(["xwc", "-j", "0"]).unwrap_err();
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
                chars: false,
                max_line_length: false,
            },
        )
        .unwrap();

        assert_eq!(
            counts,
            Counts {
                lines: 0,
                words: 0,
                chars: 0,
                bytes: 14,
                max_line_length: 0
            }
        );
    }

    #[test]
    fn longest_line_selects_only_longest_line_without_default_headings() {
        let config = Cli::try_parse_from(["xwc", "--longest-line"])
            .unwrap()
            .into_config();

        assert_eq!(
            config,
            Config {
                show_lines: false,
                show_words: false,
                show_chars: false,
                show_bytes: false,
                show_max_line_length: true,
                show_headings: false,
                human_readable: false,
                self_profile: false,
                jobs: None,
                sort_by: None,
                sort_order: SortOrder::Asc,
                globs: Vec::new(),
                files: Vec::new()
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
            show_chars: false,
            show_bytes: true,
            show_max_line_length: false,
            show_headings: true,
            human_readable: false,
            self_profile: false,
            jobs: None,
            sort_by: None,
            sort_order: SortOrder::Asc,
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
            show_chars: false,
            show_bytes: true,
            show_max_line_length: false,
            show_headings: true,
            human_readable: false,
            self_profile: false,
            jobs: None,
            sort_by: None,
            sort_order: SortOrder::Asc,
            globs: vec!["missing-*".to_owned()],
            files: Vec::new(),
        };

        assert_eq!(
            input_paths(&config).unwrap_err(),
            "missing-*: no matches".to_owned()
        );
    }

    #[test]
    fn sort_rows_keeps_total_out_of_sorted_rows() {
        let mut rows = vec![
            OutputRow {
                counts: Counts {
                    lines: 2,
                    words: 0,
                    chars: 0,
                    bytes: 20,
                    max_line_length: 0,
                },
                duration: None,
                label: Some("b.txt"),
            },
            OutputRow {
                counts: Counts {
                    lines: 1,
                    words: 0,
                    chars: 0,
                    bytes: 10,
                    max_line_length: 0,
                },
                duration: None,
                label: Some("a.txt"),
            },
        ];
        let config = Config {
            show_lines: true,
            show_words: false,
            show_chars: false,
            show_bytes: true,
            show_max_line_length: false,
            show_headings: true,
            human_readable: false,
            self_profile: false,
            jobs: None,
            sort_by: Some(SortBy::Bytes),
            sort_order: SortOrder::Desc,
            globs: Vec::new(),
            files: Vec::new(),
        };

        sort_rows(&config, &mut rows);

        assert_eq!(
            rows.into_iter()
                .map(|row| row.label.unwrap())
                .collect::<Vec<_>>(),
            vec!["b.txt", "a.txt"]
        );
    }

    #[test]
    fn sort_rows_can_sort_by_duration() {
        let mut rows = vec![
            OutputRow {
                counts: Counts {
                    lines: 2,
                    words: 0,
                    chars: 0,
                    bytes: 20,
                    max_line_length: 0,
                },
                duration: Some(Duration::from_millis(20)),
                label: Some("slow.txt"),
            },
            OutputRow {
                counts: Counts {
                    lines: 1,
                    words: 0,
                    chars: 0,
                    bytes: 10,
                    max_line_length: 0,
                },
                duration: Some(Duration::from_millis(10)),
                label: Some("fast.txt"),
            },
        ];
        let config = Config {
            show_lines: true,
            show_words: false,
            show_chars: false,
            show_bytes: true,
            show_max_line_length: false,
            show_headings: true,
            human_readable: false,
            self_profile: true,
            jobs: None,
            sort_by: Some(SortBy::Duration),
            sort_order: SortOrder::Asc,
            globs: Vec::new(),
            files: Vec::new(),
        };

        sort_rows(&config, &mut rows);

        assert_eq!(
            rows.into_iter()
                .map(|row| row.label.unwrap())
                .collect::<Vec<_>>(),
            vec!["fast.txt", "slow.txt"]
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
                chars: false,
                max_line_length: false,
            },
            Some(2),
            false,
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
                .map(|file_count| file_count.result.unwrap().counts)
                .collect::<Vec<_>>(),
            vec![
                Counts {
                    lines: 2,
                    words: 2,
                    chars: 0,
                    bytes: 8,
                    max_line_length: 0,
                },
                Counts {
                    lines: 1,
                    words: 2,
                    chars: 0,
                    bytes: 11,
                    max_line_length: 0,
                },
            ]
        );
    }
}
