use crate::CountOptions;

#[derive(Debug, Eq, PartialEq)]
pub struct Config {
    pub show_lines: bool,
    pub show_words: bool,
    pub show_bytes: bool,
    pub show_headings: bool,
    pub human_readable: bool,
    pub jobs: Option<usize>,
    pub globs: Vec<String>,
    pub files: Vec<String>,
}

impl Config {
    pub fn count_options(&self) -> CountOptions {
        CountOptions {
            lines: self.show_lines,
            words: self.show_words,
        }
    }
}
