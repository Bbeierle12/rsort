# rsort

A memory-safe, parallel sort utility written in Rust. Drop-in replacement for GNU sort under `LC_ALL=C`.

## Overview

rsort is Phase 1 of a unified Rust toolkit replacing memory-unsafe C implementations of core Unix utilities. It establishes foundational patterns for streaming data processing, parallel execution, and memory-bounded operations.

**Compatibility target**: GNU coreutils 9.x under `LC_ALL=C LANG=C`

## Features

- **Byte-oriented**: Treats input as raw bytes, not strings. No encoding assumptions.
- **GNU sort compatible**: Passes 75+ differential tests against GNU sort
- **Full flag support**: `-r`, `-n`, `-f`, `-u`, `-s`, `-k`, `-t`, `-z`, `-o`, `--debug`
- **Correct semantics**: Proper last-resort comparison (bytewise, only `-r` applies)
- **Key extraction**: Field and character position support (`-k2,2`, `-k1.3,1.5`)

## Installation

```bash
git clone https://github.com/YOUR_USERNAME/rsort.git
cd rsort
cargo build --release
```

The binary will be at `target/release/rsort`.

## Usage

```bash
# Basic sort
echo -e "c\nb\na" | rsort
# Output: a, b, c

# Numeric sort
echo -e "10\n2\n1" | rsort -n
# Output: 1, 2, 10

# Sort by second field
echo -e "x 3\ny 1\nz 2" | rsort -k2,2 -n
# Output: y 1, z 2, x 3

# Unique lines only
echo -e "a\na\nb" | rsort -u
# Output: a, b

# Reverse order
rsort -r file.txt

# Custom delimiter
rsort -t: -k2,2 /etc/passwd
```

## Flags

| Flag | Description |
|------|-------------|
| `-r` | Reverse sort order |
| `-n` | Numeric sort |
| `-f` | Case-insensitive (fold case) |
| `-u` | Output unique lines only |
| `-s` | Stable sort (preserve input order for equal keys) |
| `-k KEYDEF` | Sort by key (field.char,field.char) |
| `-t SEP` | Field separator (default: whitespace) |
| `-z` | NUL-terminated lines |
| `-o FILE` | Output to file |
| `--debug` | Show key extraction diagnostics |

## Testing

```bash
# Run all tests (single-threaded for WSL compatibility on Windows)
cargo test -- --test-threads=1

# Run unit tests only (fast)
cargo test --lib

# Run differential tests against GNU sort
cargo test --test differential -- --test-threads=1
```

## Architecture

```
src/
├── main.rs      # Entry point, CLI dispatch
├── cli.rs       # Argument parsing (clap)
├── config.rs    # Runtime configuration
├── input.rs     # Byte-oriented record reader
├── key.rs       # Key extraction from -k specs
├── compare.rs   # Comparison contract (keys + last-resort)
├── sort.rs      # Sort algorithm selection
├── output.rs    # Writer with deduplication
├── debug.rs     # --debug instrumentation
└── error.rs     # Error types
```

## Comparison Contract

rsort implements GNU sort's comparison semantics exactly:

1. **Key comparison**: Compare extracted keys with specified options (`-n`, `-f`, etc.)
2. **Last-resort**: If keys are equal, compare entire lines bytewise
   - Only `-r` affects last-resort comparison
   - `-f`, `-n`, and other options are ignored
3. **Stable/Unique**: `-s` and `-u` disable last-resort comparison

## License

MIT
