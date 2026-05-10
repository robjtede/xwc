use crate::CountOptions;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SortBy {
    Lines,
    Words,
    Chars,
    MaxLine,
    Bytes,
    File,
    Duration,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SortOrder {
    Asc,
    Desc,
}

#[derive(Debug, Eq, PartialEq)]
pub struct Config {
    pub show_lines: bool,
    pub show_words: bool,
    pub show_chars: bool,
    pub show_bytes: bool,
    pub show_max_line_length: bool,
    pub show_headings: bool,
    pub human_readable: bool,
    pub self_profile: bool,
    pub jobs: Option<usize>,
    pub sort_by: Option<SortBy>,
    pub sort_order: SortOrder,
    pub globs: Vec<String>,
    pub files: Vec<String>,
}

impl Config {
    pub fn count_options(&self) -> CountOptions {
        CountOptions {
            lines: self.show_lines,
            words: self.show_words,
            chars: self.show_chars,
            max_line_length: self.show_max_line_length,
        }
    }
}
