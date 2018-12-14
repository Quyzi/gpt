

use simplelog;

use uuid;

use gpt::disk;
use gpt::header::{read_header, write_header, Header};
use gpt::partition::{read_partitions, Partition, PartitionType};
use simplelog::{Config, SimpleLogger};
use std::io::Write;
use std::path::Path;
use std::str::FromStr;
use tempfile::NamedTempFile;

#[test]
fn test_read_header() {
    let expected_header = Header {
        signature: "EFI PART".to_string(),
        revision: 65536,
        header_size_le: 92,
        crc32: 1050019802,
        reserved: 0,
        current_lba: 1,
        backup_lba: 95,
        first_usable: 34,
        last_usable: 62,
        disk_guid: uuid::Uuid::from_str("f12fc858-c753-41d3-93a4-bfac001cdf9f").unwrap(),
        part_start: 2,
        num_parts: 128,
        part_size: 128,
        crc32_parts: 151952294,
    };

    let expected_partition = Partition {
        part_type_guid: PartitionType {
            guid: uuid::Uuid::parse_str("0FC63DAF-8483-4772-8E79-3D69D8477DE4").unwrap(),
            os: "Linux".to_string(),
            description: "Linux Filesystem Data".to_string(),
        },
        part_guid: uuid::Uuid::from_str("6fcc8240-3985-4840-901f-a05e7fd9b69d").unwrap(),
        first_lba: 34,
        last_lba: 62,
        flags: 0,
        name: "primary".to_string(),
    };

    let diskpath = Path::new("tests/fixtures/gpt-linux-disk-01.img");
    let h = read_header(diskpath, disk::DEFAULT_SECTOR_SIZE).unwrap();

    println!("header: {:?}", h);
    assert_eq!(h, expected_header);

    let p = read_partitions(diskpath, &h, disk::DEFAULT_SECTOR_SIZE).unwrap();
    println!("Partitions: {:?}", p);
    assert_eq!(p[0], expected_partition);
}

#[test]
fn test_write_header() {
    let _ = SimpleLogger::init(simplelog::LevelFilter::Trace, Config::default());
    let mut tempdisk = NamedTempFile::new().expect("failed to create tempfile disk");
    {
        let data: [u8; 4096] = [0; 4096];
        println!("Creating blank header file for testing");
        for _ in 0..100 {
            tempdisk.as_file_mut().write_all(&data).unwrap();
        }
    };
    println!("Writing header");
    let w = write_header(
        tempdisk.path(),
        Some(uuid::Uuid::from_str("f400b934-48ef-4381-8f26-459f6b33c7df").unwrap()),
        disk::DEFAULT_SECTOR_SIZE,
    );
    println!("Wrote header: {:?}", w);
    println!("Reading header");
    let h = read_header(tempdisk.path(), disk::DEFAULT_SECTOR_SIZE).unwrap();
    println!("header: {:#?}", h);

    let p = Partition {
        part_type_guid: PartitionType {
            guid: uuid::Uuid::parse_str("0FC63DAF-8483-4772-8E79-3D69D8477DE4").unwrap(),
            os: "Linux".to_string(),
            description: "Linux Filesystem Data".to_string(),
        },
        part_guid: uuid::Uuid::new_v4(),
        first_lba: 36,
        last_lba: 40,
        flags: 0,
        name: "gpt test".to_string(),
    };
    p.write(tempdisk.path(), &h, disk::DEFAULT_SECTOR_SIZE)
        .unwrap();
}
