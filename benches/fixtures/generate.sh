#!/usr/bin/env bash

set -eEuo pipefail

fixture_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"

fixture_sizes="${XWC_FIXTURE_SIZES-1K 10K 100K 1M 10M 100M 500M}"
long_line_fixture_sizes="${XWC_LONG_LINE_FIXTURE_SIZES-1K 10K 100K 1M 10M 100M 500M}"
chunk_size="${XWC_CHUNK_SIZE:-65536}"
block_size="${XWC_BLOCK_SIZE:-1048576}"

if ((chunk_size < 4)); then
    printf 'XWC_CHUNK_SIZE must be at least 4\n' >&2
    exit 1
fi

if ((block_size < chunk_size)); then
    printf 'XWC_BLOCK_SIZE must be at least XWC_CHUNK_SIZE\n' >&2
    exit 1
fi

parse_size() {
    local spec="$1"
    local number suffix multiplier

    suffix="${spec: -1}"
    case "$suffix" in
        K | k)
            number="${spec%?}"
            multiplier=1024
            ;;
        M | m)
            number="${spec%?}"
            multiplier=$((1024 * 1024))
            ;;
        G | g)
            number="${spec%?}"
            multiplier=$((1024 * 1024 * 1024))
            ;;
        *)
            number="$spec"
            multiplier=1
            ;;
    esac

    case "$number" in
        '' | *[!0-9]*)
            printf 'Invalid fixture size: %s\n' "$spec" >&2
            exit 1
            ;;
    esac

    printf '%s\n' "$((number * multiplier))"
}

size_label() {
    printf '%s' "$1" | tr '[:upper:]' '[:lower:]'
}

file_size() {
    local size

    size="$(wc -c <"$1")"
    printf '%s\n' "${size//[[:space:]]/}"
}

write_a_file() {
    local path="$1"
    local byte_count="$2"

    dd if=/dev/zero bs="$byte_count" count=1 2>/dev/null | tr '\0' 'a' >"$path"
}

append_file_limited() {
    local source="$1"
    local destination="$2"
    local source_size

    if ((remaining == 0)); then
        return
    fi

    source_size="$(file_size "$source")"
    if ((remaining >= source_size)); then
        cat "$source" >>"$destination"
        remaining=$((remaining - source_size))
    else
        dd if="$source" bs="$remaining" count=1 2>/dev/null >>"$destination"
        remaining=0
    fi
}

make_repeating_block() {
    local template="$1"
    local block="$2"
    local work
    local template_size

    template_size="$(file_size "$template")"
    if ((template_size == 0)); then
        printf 'Template must not be empty: %s\n' "$template" >&2
        exit 1
    fi

    work="$tmp_dir/$(basename "$block").work"
    cp "$template" "$work"
    while (("$(file_size "$work")" < block_size)); do
        cat "$work" "$work" >"$block"
        mv "$block" "$work"
    done
    dd if="$work" of="$block" bs="$block_size" count=1 2>/dev/null
}

write_repeated_fixture() {
    local name="$1"
    local block="$2"
    local byte_count="$3"
    local path="$fixture_dir/$name"

    printf 'Generating %s (%s bytes)\n' "$name" "$byte_count"
    remaining="$byte_count"
    : >"$path"
    while ((remaining > 0)); do
        append_file_limited "$block" "$path"
    done
}

write_split_valid_utf8_fixture() {
    local name="$1"
    local byte_count="$2"
    local path="$fixture_dir/$name"

    printf 'Generating %s (%s bytes)\n' "$name" "$byte_count"
    remaining="$byte_count"
    : >"$path"
    append_file_limited "$valid_split_first_chunk" "$path"
    while ((remaining > 0)); do
        append_file_limited "$valid_split_block" "$path"
    done
}

write_split_invalid_utf8_fixture() {
    local name="$1"
    local byte_count="$2"
    local path="$fixture_dir/$name"

    printf 'Generating %s (%s bytes)\n' "$name" "$byte_count"
    remaining="$byte_count"
    : >"$path"
    while ((remaining > 0)); do
        append_file_limited "$invalid_split_block" "$path"
    done
}

write_random_base64_long_line_fixture() {
    local name="$1"
    local byte_count="$2"
    local path="$fixture_dir/$name"
    local raw_byte_count actual_size

    if ((byte_count % 4 != 0)); then
        printf 'Random base64 long-line fixture size must be divisible by 4: %s bytes\n' "$byte_count" >&2
        exit 1
    fi

    raw_byte_count="$((byte_count * 3 / 4))"

    printf 'Generating %s (%s bytes)\n' "$name" "$byte_count"
    dd if=/dev/urandom bs="$raw_byte_count" count=1 2>/dev/null | base64 | tr -d '\n' >"$path"

    actual_size="$(file_size "$path")"
    if ((actual_size != byte_count)); then
        printf 'Generated %s with %s bytes, expected %s bytes\n' "$name" "$actual_size" "$byte_count" >&2
        exit 1
    fi
}

tmp_dir="$(mktemp -d)"
trap 'rm -rf "$tmp_dir"' EXIT

printf 'Preparing fixture templates\n'

ascii_template="$tmp_dir/ascii-template"
mixed_template="$tmp_dir/mixed-template"
emoji_template="$tmp_dir/emoji-template"
cjk_template="$tmp_dir/cjk-template"
invalid_latin1_template="$tmp_dir/invalid-latin1-template"

printf 'hello world this is ascii text\n' >"$ascii_template"
printf 'cafe café 東京 京都 emoji 😀\n' >"$mixed_template"
printf '😀 😁 😂 🤖 🚀 cafe\n' >"$emoji_template"
printf '東京 京都 大阪 札幌 福岡\n' >"$cjk_template"
printf 'caf\351 cr\350me jalape\361o\n' >"$invalid_latin1_template"

ascii_block="$tmp_dir/ascii-block"
mixed_block="$tmp_dir/mixed-block"
emoji_block="$tmp_dir/emoji-block"
cjk_block="$tmp_dir/cjk-block"
invalid_latin1_block="$tmp_dir/invalid-latin1-block"

make_repeating_block "$ascii_template" "$ascii_block"
make_repeating_block "$mixed_template" "$mixed_block"
make_repeating_block "$emoji_template" "$emoji_block"
make_repeating_block "$cjk_template" "$cjk_block"
make_repeating_block "$invalid_latin1_template" "$invalid_latin1_block"

valid_prefix="$tmp_dir/valid-prefix-a"
valid_middle="$tmp_dir/valid-middle-a"
invalid_prefix="$tmp_dir/invalid-prefix-a"
valid_split_first_chunk="$tmp_dir/valid-split-first-chunk"
valid_split_chunk="$tmp_dir/valid-split-chunk"
valid_split_block="$tmp_dir/valid-split-block"
invalid_split_chunk="$tmp_dir/invalid-split-chunk"
invalid_split_block="$tmp_dir/invalid-split-block"

write_a_file "$valid_prefix" "$((chunk_size - 2))"
write_a_file "$valid_middle" "$((chunk_size - 3))"
write_a_file "$invalid_prefix" "$((chunk_size - 1))"

cat "$valid_prefix" >"$valid_split_first_chunk"
printf '\342\202' >>"$valid_split_first_chunk"

: >"$valid_split_chunk"
printf '\254' >>"$valid_split_chunk"
cat "$valid_middle" >>"$valid_split_chunk"
printf '\342\202' >>"$valid_split_chunk"
make_repeating_block "$valid_split_chunk" "$valid_split_block"

cat "$invalid_prefix" >"$invalid_split_chunk"
printf '\342' >>"$invalid_split_chunk"
make_repeating_block "$invalid_split_chunk" "$invalid_split_block"

for size in $fixture_sizes; do
    bytes="$(parse_size "$size")"
    label="$(size_label "$size")"

    printf 'Generating %s fixture set\n' "$size"
    write_repeated_fixture "ascii-words-$label.txt" "$ascii_block" "$bytes"
    write_repeated_fixture "mixed-utf8-$label.txt" "$mixed_block" "$bytes"
    write_repeated_fixture "emoji-heavy-utf8-$label.txt" "$emoji_block" "$bytes"
    write_repeated_fixture "cjk-utf8-$label.txt" "$cjk_block" "$bytes"
    write_repeated_fixture "invalid-latin1-bytes-$label.txt" "$invalid_latin1_block" "$bytes"
    write_split_valid_utf8_fixture "split-valid-utf8-$label.txt" "$bytes"
    write_split_invalid_utf8_fixture "split-invalid-utf8-$label.txt" "$bytes"
done

for size in $long_line_fixture_sizes; do
    bytes="$(parse_size "$size")"
    label="$(size_label "$size")"

    write_random_base64_long_line_fixture "random-base64-long-line-$label.txt" "$bytes"
done

printf 'Generated fixture sizes: %s\n' "$fixture_sizes"
printf 'Generated long-line fixture sizes: %s\n' "$long_line_fixture_sizes"
printf 'Generated fixtures in %s\n' "$fixture_dir"
