use std::time::Duration;

use bytesize::ByteSize;

use crate::{Config, Counts};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OutputRow<'a> {
    pub counts: Counts,
    pub duration: Option<Duration>,
    pub label: Option<&'a str>,
}

pub fn render_rows<'a>(
    config: &Config,
    rows: impl IntoIterator<Item = OutputRow<'a>>,
) -> Vec<Vec<String>> {
    let rows = rows.into_iter().collect::<Vec<_>>();
    let has_labels = rows.iter().any(|row| row.label.is_some());
    let mut rendered_rows = Vec::new();

    if config.show_headings {
        rendered_rows.push(headings(config, has_labels));
    }

    for row in rows {
        rendered_rows.push(fields(config, row));
    }

    rendered_rows
}

pub fn column_widths(rows: &[Vec<String>]) -> Vec<usize> {
    let column_count = rows.iter().map(Vec::len).max().unwrap_or(0);
    let mut widths = vec![0; column_count];

    for row in rows {
        for (width, field) in widths.iter_mut().zip(row) {
            *width = (*width).max(field.len());
        }
    }

    widths
}

pub fn format_byte_count(bytes: u64, human_readable: bool) -> String {
    if human_readable {
        ByteSize::b(bytes).display().iec_short().to_string()
    } else {
        bytes.to_string()
    }
}

pub fn format_duration(duration: Duration) -> String {
    let nanos = duration.as_nanos();

    if nanos < 1_000 {
        format!("{nanos}ns")
    } else if nanos < 1_000_000 {
        format_tenths(nanos, 1_000, "us")
    } else if nanos < 1_000_000_000 {
        format_tenths(nanos, 1_000_000, "ms")
    } else {
        let mut seconds = u128::from(duration.as_secs());
        let mut millis = u128::from(duration.subsec_nanos()) / 1_000_000;

        if millis == 1_000 {
            seconds += 1;
            millis = 0;
        }

        format!("{seconds}.{millis:03}s")
    }
}

fn format_tenths(nanos: u128, divisor: u128, unit: &str) -> String {
    let tenths = (nanos * 10 + divisor / 2) / divisor;
    let whole = tenths / 10;
    let fraction = tenths % 10;

    format!("{whole}.{fraction}{unit}")
}

fn headings(config: &Config, has_labels: bool) -> Vec<String> {
    let mut fields = Vec::new();

    if config.show_lines {
        fields.push("lines".to_owned());
    }

    if config.show_words {
        fields.push("words".to_owned());
    }

    if config.show_chars {
        fields.push("chars".to_owned());
    }

    if config.show_max_line_length {
        fields.push("max-line".to_owned());
    }

    if config.show_bytes {
        fields.push(byte_heading(config).to_owned());
    }

    if has_labels {
        fields.push("file".to_owned());
    }

    if config.self_profile {
        fields.push("duration".to_owned());
    }

    fields
}

fn fields(config: &Config, row: OutputRow<'_>) -> Vec<String> {
    let mut fields = Vec::new();

    if config.show_lines {
        fields.push(row.counts.lines.to_string());
    }

    if config.show_words {
        fields.push(row.counts.words.to_string());
    }

    if config.show_chars {
        fields.push(row.counts.chars.to_string());
    }

    if config.show_max_line_length {
        fields.push(row.counts.max_line_length.to_string());
    }

    if config.show_bytes {
        fields.push(format_byte_count(row.counts.bytes, config.human_readable));
    }

    if let Some(label) = row.label {
        fields.push(label.to_owned());
    }

    if config.self_profile {
        fields.push(row.duration.map(format_duration).unwrap_or_default());
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SortOrder;

    fn default_config() -> Config {
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
            files: Vec::new(),
        }
    }

    #[test]
    fn renders_default_headings_and_fields() {
        let rendered = render_rows(
            &default_config(),
            [OutputRow {
                counts: Counts {
                    lines: 2,
                    words: 0,
                    chars: 0,
                    bytes: 14,
                    max_line_length: 0,
                },
                duration: None,
                label: None,
            }],
        );

        assert_eq!(
            rendered,
            vec![
                vec!["lines".to_owned(), "bytes".to_owned()],
                vec!["2".to_owned(), "14".to_owned()],
            ]
        );
    }

    #[test]
    fn renders_label_heading_when_any_row_has_label() {
        let rendered = render_rows(
            &default_config(),
            [OutputRow {
                counts: Counts {
                    lines: 2,
                    words: 0,
                    chars: 0,
                    bytes: 14,
                    max_line_length: 0,
                },
                duration: None,
                label: Some("a.txt"),
            }],
        );

        assert_eq!(
            rendered,
            vec![
                vec!["lines".to_owned(), "bytes".to_owned(), "file".to_owned()],
                vec!["2".to_owned(), "14".to_owned(), "a.txt".to_owned()],
            ]
        );
    }

    #[test]
    fn formats_human_readable_bytes() {
        assert_eq!(format_byte_count(1024, true), "1.0K");
        assert_eq!(format_byte_count(1024, false), "1024");
    }

    #[test]
    fn renders_duration_column_when_self_profiled() {
        let mut config = default_config();
        config.self_profile = true;
        let rendered = render_rows(
            &config,
            [OutputRow {
                counts: Counts {
                    lines: 2,
                    words: 0,
                    chars: 0,
                    bytes: 14,
                    max_line_length: 0,
                },
                duration: Some(Duration::from_micros(12)),
                label: Some("a.txt"),
            }],
        );

        assert_eq!(
            rendered,
            vec![
                vec![
                    "lines".to_owned(),
                    "bytes".to_owned(),
                    "file".to_owned(),
                    "duration".to_owned()
                ],
                vec![
                    "2".to_owned(),
                    "14".to_owned(),
                    "a.txt".to_owned(),
                    "12.0us".to_owned()
                ],
            ]
        );
    }
}
