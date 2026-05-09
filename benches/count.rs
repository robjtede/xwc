use std::{hint::black_box, sync::LazyLock};

use divan::Bencher;
use xwc::{CountOptions, count_reader};

static SMALL_ASCII: &[u8] = b"one two\nthree four\nfive\n";
static MIXED_UTF8: &str = "cafe\ncafé\n東京 京都\nemoji 😀 test\n";
static LARGE_ASCII: LazyLock<Vec<u8>> =
    LazyLock::new(|| "alpha beta gamma delta\n".repeat(16 * 1024).into_bytes());

mod words_excluded {
    use super::*;

    const OPTIONS: CountOptions = CountOptions { words: false };

    #[divan::bench]
    fn count_small_ascii(bencher: Bencher) {
        bencher.bench_local(|| black_box(count_reader(black_box(SMALL_ASCII), OPTIONS).unwrap()));
    }

    #[divan::bench]
    fn count_mixed_utf8(bencher: Bencher) {
        bencher.bench_local(|| {
            black_box(count_reader(black_box(MIXED_UTF8.as_bytes()), OPTIONS).unwrap())
        });
    }

    #[divan::bench]
    fn count_large_ascii(bencher: Bencher) {
        bencher.bench_local(|| {
            black_box(count_reader(black_box(LARGE_ASCII.as_slice()), OPTIONS).unwrap())
        });
    }
}

mod words_included {
    use super::*;

    const OPTIONS: CountOptions = CountOptions { words: true };

    #[divan::bench]
    fn count_small_ascii(bencher: Bencher) {
        bencher.bench_local(|| black_box(count_reader(black_box(SMALL_ASCII), OPTIONS).unwrap()));
    }

    #[divan::bench]
    fn count_mixed_utf8(bencher: Bencher) {
        bencher.bench_local(|| {
            black_box(count_reader(black_box(MIXED_UTF8.as_bytes()), OPTIONS).unwrap())
        });
    }

    #[divan::bench]
    fn count_large_ascii(bencher: Bencher) {
        bencher.bench_local(|| {
            black_box(count_reader(black_box(LARGE_ASCII.as_slice()), OPTIONS).unwrap())
        });
    }
}

fn main() {
    divan::main();
}
