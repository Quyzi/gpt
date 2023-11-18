use gpt;

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
