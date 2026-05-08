use std::{hint::black_box, sync::LazyLock};

use divan::Bencher;
use xwc::count_reader;

fn main() {
    divan::main();
}

#[divan::bench]
fn count_small_ascii(bencher: Bencher) {
    bencher.bench_local(|| count_reader(black_box(SMALL_ASCII)).unwrap());
}

#[divan::bench]
fn count_mixed_utf8(bencher: Bencher) {
    bencher.bench_local(|| count_reader(black_box(MIXED_UTF8.as_bytes())).unwrap());
}

#[divan::bench]
fn count_large_ascii(bencher: Bencher) {
    bencher.bench_local(|| count_reader(black_box(LARGE_ASCII.as_slice())).unwrap());
}

static SMALL_ASCII: &[u8] = b"one two\nthree four\nfive\n";
static MIXED_UTF8: &str = "cafe\ncafé\n東京 京都\nemoji 😀 test\n";
static LARGE_ASCII: LazyLock<Vec<u8>> =
    LazyLock::new(|| "alpha beta gamma delta\n".repeat(16 * 1024).into_bytes());
