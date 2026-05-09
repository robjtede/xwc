use std::io::{self, Read};

use memchr::memchr_iter;

const BUFFER_SIZE: usize = 64 * 1024;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Counts {
    pub lines: u64,
    pub words: u64,
    pub bytes: u64,
}

#[derive(Debug, Default)]
struct WordState {
    in_word: bool,
    pending_utf8: Vec<u8>,
}

pub fn count_reader(mut reader: impl Read) -> io::Result<Counts> {
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
    memchr_iter(b'\n', buffer).count()
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
}
