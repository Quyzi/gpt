use gpt::{disk, partition_types, GptConfig};

use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{self};
use tempfile::NamedTempFile;

#[test]
fn test_gpt_disk_read() {
    let diskpath = path::Path::new("tests/fixtures/gpt-disk.img");
    let lb_size = disk::LogicalBlockSize::Lb512;

    let gdisk = GptConfig::new().open(diskpath).unwrap();
    assert_eq!(*gdisk.logical_block_size(), lb_size);
    assert!(gdisk.primary_header().is_ok());
    assert!(gdisk.backup_header().is_ok());
    assert_eq!(gdisk.partitions().len(), 2);

    let h1 = gdisk.primary_header().unwrap();
    assert_eq!(h1.current_lba, 1);
    assert_eq!(h1.backup_lba, 71);

    let h2 = gdisk.backup_header().unwrap();
    assert_eq!(h2.current_lba, 71);
    assert_eq!(h2.backup_lba, 1);

    let p1 = gdisk.partitions().get(&1).unwrap();
    assert_eq!(p1.part_type_guid, partition_types::LINUX_FS);
    assert_eq!(
        p1.part_guid,
        "F38EAB50-076F-CB45-97F8-B1B7E5AF078F".parse().unwrap()
    );
    assert_eq!(p1.first_lba, 34);
    assert_eq!(p1.last_lba, 34);
    assert_eq!(p1.name, "");

    let p1_start = p1.bytes_start(*gdisk.logical_block_size()).unwrap();
    assert_eq!(p1_start, 512 * 34);
    let p1_len = p1.bytes_len(*gdisk.logical_block_size()).unwrap();
    assert_eq!(p1_len, 512);

    let p2 = gdisk.partitions().get(&2).unwrap();
    assert_eq!(p2.part_type_guid, partition_types::LINUX_FS);
    assert_eq!(
        p2.part_guid,
        "8EEE35AF-4A93-2C4F-AA7A-5FB193AC6FF7".parse().unwrap()
    );
    assert_eq!(p2.first_lba, 35);
    assert_eq!(p2.last_lba, 38);
    assert_eq!(p2.name, "");

    let p2_start = p2.bytes_start(*gdisk.logical_block_size()).unwrap();
    assert_eq!(p2_start, 512 * 35);
    let p2_len = p2.bytes_len(*gdisk.logical_block_size()).unwrap();
    assert_eq!(p2_len, 4 * 512);

    let part_alignment = gdisk.calculate_alignment();
    println!("Test part alignment={}", part_alignment);
    assert_eq!(part_alignment, 1);
}

#[test]
fn test_gpt_disk_write_fidelity_with_device() {
    let diskpath = path::Path::new("tests/fixtures/gpt-disk.img");

    // Assumes that test_gpt_disk has passed, no need to check answers.
    let gdisk = GptConfig::new().open(diskpath).unwrap();
    let good_header1 = gdisk.primary_header().unwrap().clone();
    let good_header2 = gdisk.backup_header().unwrap().clone();
    let good_partitions = gdisk.partitions().clone();
    println!("good header1={:?}", good_header1);
    println!("good header2={:?}", good_header2);
    println!("good partitions={:#?}", good_partitions);

    // Test that we can write this test partition table to an in-memory buffer
    // instead, then load the results and verify they should be the same.
    let image_size = usize::try_from(std::fs::metadata(diskpath).unwrap().len()).unwrap();
    let mem_device = Box::new(std::io::Cursor::new(vec![0_u8; image_size]));
    let gdisk = gdisk.with_disk_device(mem_device, true);
    let mut mem_device = gdisk.write().unwrap();

    // Write this memory buffer to a temp file, and load from the file to verify
    // that we wrote the data to the memory buffer correctly.
    let mut tempdisk = NamedTempFile::new().expect("failed to create tempfile disk");
    let mut gpt_in_mem = vec![0_u8; image_size];
    let _ = mem_device.seek(SeekFrom::Start(0)).unwrap();
    mem_device.read_exact(&mut gpt_in_mem).unwrap();
    tempdisk.write_all(&gpt_in_mem).unwrap();
    tempdisk.flush().unwrap();

    let gdisk_file = GptConfig::new().open(tempdisk.path()).unwrap();
    println!("file header1={:?}", gdisk_file.primary_header().unwrap());
    println!("file header2={:?}", gdisk_file.backup_header().unwrap());
    println!("file partitions={:#?}", gdisk_file.partitions());
    assert_eq!(gdisk_file.primary_header().unwrap(), &good_header1);
    assert_eq!(gdisk_file.backup_header().unwrap(), &good_header2);
    assert_eq!(gdisk_file.partitions().clone(), good_partitions);

    // Test that if we read it back from this memory buffer, it matches the known good.
    let gdisk_mem = GptConfig::new().open_from_device(mem_device).unwrap();
    assert_eq!(gdisk_mem.primary_header().unwrap(), &good_header1);
    assert_eq!(gdisk_mem.backup_header().unwrap(), &good_header2);
    assert_eq!(gdisk_mem.partitions().clone(), good_partitions);
}
