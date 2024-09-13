# gpt
[![crates.io](https://img.shields.io/crates/v/gpt.svg)](https://crates.io/crates/gpt)
![minimum rust 1.65](https://img.shields.io/badge/rust-1.65%2B-orange.svg)
[![Documentation](https://docs.rs/gpt/badge.svg)](https://docs.rs/gpt)

A pure-Rust library to work with GPT partition tables.

`gpt` provides support for manipulating (R/W) GPT headers and partition
tables. It supports any  that implements the `Read + Write + Seek + Debug` traits. 

## Example

```rust
use std::error::Error;

fn main() {
    // Inspect disk image, handling errors.
    if let Err(e) = run() {
        eprintln!("Failed to inspect image: {}", e);
        std::process::exit(1)
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    // First parameter is target disk image (optional, default: fixtures sample)
    let sample = "tests/fixtures/gpt-disk.img".to_string();
    let input = std::env::args().nth(1).unwrap_or(sample);

    // Open disk image.
    let cfg = gpt::GptConfig::new().writable(false);
    let disk = cfg.open(input)?;

    // Print GPT layout.
    println!("Disk (primary) header: {:#?}", disk.primary_header());
    println!("Partition layout: {:#?}", disk.partitions());

    Ok(())
}
```
