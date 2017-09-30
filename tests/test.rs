extern crate gpt;
extern crate uuid;

use std::str::FromStr;
use gpt::header::{Header, read_header};
use gpt::partition::{Partition, PartitionType, read_partitions};

#[test]
fn test_header() {
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
            os: "Linux".to_string(),
            guid: "0FC63DAF-8483-4772-8E79-3D69D8477DE4".to_string(),
            desc: "Linux Filesystem Data".to_string(),
        },
        part_guid: uuid::Uuid::from_str("6fcc8240-3985-4840-901f-a05e7fd9b69d").unwrap(),
        first_LBA: 34,
        last_LBA: 62,
        flags: 0,
        name: "primary".to_string(),
    };

    let filename = "tests/test_gpt".to_string();
    let h = read_header(&filename).unwrap();

    println!("header: {:?}", h);
    assert_eq!(h, expected_header);

    let p = read_partitions(&filename, &h).unwrap();
    println!("Partitions: {:?}", p);
    assert_eq!(p[0], expected_partition);

}
