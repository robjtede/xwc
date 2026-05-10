use std::{
    io::{self, Read},
    mem, str,
};

use memchr::memchr_iter;

const BUFFER_SIZE: usize = 64 * 1024;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Counts {
    pub lines: u64,
    pub words: u64,
    pub chars: u64,
    pub bytes: u64,
    pub max_line_length: u64,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct CountOptions {
    pub lines: bool,
    pub words: bool,
    pub chars: bool,
    pub max_line_length: bool,
}

#[derive(Debug, Default)]
struct WordState {
    in_word: bool,
    pending_utf8: Vec<u8>,
}

#[derive(Debug, Default)]
struct CharState {
    pending_utf8: Vec<u8>,
}

#[derive(Debug, Default)]
struct LineLengthState {
    pending_utf8: Vec<u8>,
    current_line_length: u64,
}

pub fn count_reader(mut reader: impl Read, options: CountOptions) -> io::Result<Counts> {
    let mut counts = Counts::default();
    let mut buffer = [0; BUFFER_SIZE];
    let mut word_state = WordState::default();
    let mut char_state = CharState::default();
    let mut line_length_state = LineLengthState::default();

    loop {
        let read = reader.read(&mut buffer)?;
        if read == 0 {
            break;
        }

        counts.bytes += read as u64;
        let chunk = buffer
            .get(..read)
            .expect("Read::read cannot report more bytes than the buffer holds");

        if options.lines {
            counts.lines += bytecount_newlines(chunk) as u64;
        }

        if options.words {
            counts.words += count_words(chunk, &mut word_state) as u64;
        }

        if options.chars {
            counts.chars += count_chars(chunk, &mut char_state) as u64;
        }

        if options.max_line_length {
            count_max_line_length(chunk, &mut line_length_state, &mut counts.max_line_length);
        }
    }

    if options.words && !word_state.pending_utf8.is_empty() && !word_state.in_word {
        counts.words += 1;
    }

    if options.chars && !char_state.pending_utf8.is_empty() {
        counts.chars += 1;
    }

    if options.max_line_length {
        if !line_length_state.pending_utf8.is_empty() {
            line_length_state.current_line_length += 1;
        }

        counts.max_line_length = counts
            .max_line_length
            .max(line_length_state.current_line_length);
    }

    Ok(counts)
}

fn bytecount_newlines(buffer: &[u8]) -> usize {
    memchr_iter(b'\n', buffer).count()
}

fn count_chars(buffer: &[u8], state: &mut CharState) -> usize {
    let mut chars = 0;
    let combined;

    let buffer = if state.pending_utf8.is_empty() {
        buffer
    } else {
        combined = {
            let mut bytes = mem::take(&mut state.pending_utf8);
            bytes.extend_from_slice(buffer);
            bytes
        };
        &combined
    };

    let mut offset = 0;

    while offset < buffer.len() {
        let remaining = buffer
            .get(offset..)
            .expect("offset is guarded by the loop condition");

        match str::from_utf8(remaining) {
            Ok(valid) => {
                chars += valid.chars().count();
                break;
            }
            Err(error) => {
                let valid_end = offset + error.valid_up_to();
                let valid_bytes = buffer
                    .get(offset..valid_end)
                    .expect("valid_up_to returns an in-bounds offset");
                let valid = str::from_utf8(valid_bytes)
                    .expect("valid_up_to must split at a UTF-8 boundary");
                chars += valid.chars().count();
                offset = valid_end;

                if let Some(error_len) = error.error_len() {
                    chars += 1;
                    offset += error_len;
                } else {
                    let pending = buffer
                        .get(offset..)
                        .expect("offset is guarded by the loop condition");
                    state.pending_utf8.extend_from_slice(pending);
                    break;
                }
            }
        }
    }

    chars
}

fn count_words(buffer: &[u8], state: &mut WordState) -> usize {
    let mut words = 0;
    let combined;

    let buffer = if state.pending_utf8.is_empty() {
        buffer
    } else {
        combined = {
            let mut bytes = mem::take(&mut state.pending_utf8);
            bytes.extend_from_slice(buffer);
            bytes
        };
        &combined
    };

    let mut offset = 0;

    while offset < buffer.len() {
        let remaining = buffer
            .get(offset..)
            .expect("offset is guarded by the loop condition");

        match str::from_utf8(remaining) {
            Ok(valid) => {
                words += count_words_in_str(valid, &mut state.in_word);
                break;
            }
            Err(error) => {
                let valid_end = offset + error.valid_up_to();
                let valid_bytes = buffer
                    .get(offset..valid_end)
                    .expect("valid_up_to returns an in-bounds offset");
                let valid = str::from_utf8(valid_bytes)
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
                    let pending = buffer
                        .get(offset..)
                        .expect("offset is guarded by the loop condition");
                    state.pending_utf8.extend_from_slice(pending);
                    break;
                }
            }
        }
    }

    words
}

fn count_max_line_length(buffer: &[u8], state: &mut LineLengthState, max_line_length: &mut u64) {
    let combined;

    let buffer = if state.pending_utf8.is_empty() {
        buffer
    } else {
        combined = {
            let mut bytes = mem::take(&mut state.pending_utf8);
            bytes.extend_from_slice(buffer);
            bytes
        };
        &combined
    };

    let mut offset = 0;

    while offset < buffer.len() {
        let remaining = buffer
            .get(offset..)
            .expect("offset is guarded by the loop condition");

        match str::from_utf8(remaining) {
            Ok(valid) => {
                update_max_line_length(valid, state, max_line_length);
                break;
            }
            Err(error) => {
                let valid_end = offset + error.valid_up_to();
                let valid_bytes = buffer
                    .get(offset..valid_end)
                    .expect("valid_up_to returns an in-bounds offset");
                let valid = str::from_utf8(valid_bytes)
                    .expect("valid_up_to must split at a UTF-8 boundary");
                update_max_line_length(valid, state, max_line_length);
                offset = valid_end;

                if let Some(error_len) = error.error_len() {
                    state.current_line_length += 1;
                    offset += error_len;
                } else {
                    let pending = buffer
                        .get(offset..)
                        .expect("offset is guarded by the loop condition");
                    state.pending_utf8.extend_from_slice(pending);
                    break;
                }
            }
        }
    }
}

fn update_max_line_length(input: &str, state: &mut LineLengthState, max_line_length: &mut u64) {
    for ch in input.chars() {
        if ch == '\n' {
            *max_line_length = (*max_line_length).max(state.current_line_length);
            state.current_line_length = 0;
        } else {
            state.current_line_length += 1;
        }
    }
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

impl std::ops::AddAssign for Counts {
    fn add_assign(&mut self, rhs: Self) {
        self.lines += rhs.lines;
        self.words += rhs.words;
        self.chars += rhs.chars;
        self.bytes += rhs.bytes;
        self.max_line_length = self.max_line_length.max(rhs.max_line_length);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_newlines_words_chars_and_bytes() {
        let input = "cafe\ncafé\n東京 京都".as_bytes();

        assert_eq!(
            count_reader(
                input,
                CountOptions {
                    lines: true,
                    words: true,
                    chars: true,
                    max_line_length: true
                }
            )
            .unwrap(),
            Counts {
                lines: 2,
                words: 4,
                chars: 15,
                bytes: 24,
                max_line_length: 5
            }
        );
    }

    #[test]
    fn skips_word_counting_when_words_are_not_requested() {
        let input = "cafe\ncafé\n東京 京都".as_bytes();

        assert_eq!(
            count_reader(
                input,
                CountOptions {
                    lines: true,
                    words: false,
                    chars: false,
                    max_line_length: false
                }
            )
            .unwrap(),
            Counts {
                lines: 2,
                words: 0,
                chars: 0,
                bytes: 24,
                max_line_length: 0
            }
        );
    }

    #[test]
    fn skips_line_counting_when_lines_are_not_requested() {
        let input = "one\ntwo\nthree\n".as_bytes();

        assert_eq!(
            count_reader(
                input,
                CountOptions {
                    lines: false,
                    words: false,
                    chars: false,
                    max_line_length: false
                }
            )
            .unwrap(),
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
    fn counts_max_line_length() {
        let input = "one\nthree\ncafé\n東京 京都".as_bytes();

        assert_eq!(
            count_reader(
                input,
                CountOptions {
                    lines: false,
                    words: false,
                    chars: false,
                    max_line_length: true
                }
            )
            .unwrap(),
            Counts {
                lines: 0,
                words: 0,
                chars: 0,
                bytes: 29,
                max_line_length: 5
            }
        );
    }

    #[test]
    fn counts_max_line_length_across_buffer_boundaries() {
        let mut state = LineLengthState::default();
        let input = "ab\ncafé\n東京 京都".as_bytes();
        let mut max_line_length = 0;

        count_max_line_length(&input[..6], &mut state, &mut max_line_length);
        count_max_line_length(&input[6..], &mut state, &mut max_line_length);
        max_line_length = max_line_length.max(state.current_line_length);

        assert_eq!(max_line_length, 5);
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
    fn counts_utf8_chars_across_buffer_boundaries() {
        let mut state = CharState::default();
        let input = "東京 京都".as_bytes();

        assert_eq!(count_chars(&input[..4], &mut state), 1);
        assert_eq!(count_chars(&input[4..], &mut state), 4);
    }
}
