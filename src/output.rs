use bytesize::ByteSize;

use crate::{Config, Counts};

pub fn render_rows<'a>(
    config: &Config,
    rows: impl IntoIterator<Item = (Counts, Option<&'a str>)>,
) -> Vec<Vec<String>> {
    let rows = rows.into_iter().collect::<Vec<_>>();
    let has_labels = rows.iter().any(|(_, label)| label.is_some());
    let mut rendered_rows = Vec::new();

    if config.show_headings {
        rendered_rows.push(headings(config, has_labels));
    }

    for (counts, label) in rows {
        rendered_rows.push(fields(config, counts, label));
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

#[cfg(test)]
mod tests {
    use super::*;

    fn default_config() -> Config {
        Config {
            show_lines: true,
            show_words: false,
            show_bytes: true,
            show_headings: true,
            human_readable: false,
            jobs: None,
            globs: Vec::new(),
            files: Vec::new(),
        }
    }

    #[test]
    fn renders_default_headings_and_fields() {
        let rendered = render_rows(
            &default_config(),
            [(
                Counts {
                    lines: 2,
                    words: 0,
                    bytes: 14,
                },
                None,
            )],
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
            [(
                Counts {
                    lines: 2,
                    words: 0,
                    bytes: 14,
                },
                Some("a.txt"),
            )],
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
}
