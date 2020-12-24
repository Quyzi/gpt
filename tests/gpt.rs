use gpt;

use gpt::disk;
use std::convert::TryFrom;
use std::io::{SeekFrom, Write};
use std::path;
use tempfile::NamedTempFile;

#[test]
fn test_gptconfig_empty() {
    let tempdisk = NamedTempFile::new().expect("failed to create tempfile disk");
    let cfg = {
        let c1 = gpt::GptConfig::new();
        let c2 = gpt::GptConfig::default();
        assert_eq!(c1, c2);
        c1
    };

    let lb_size = disk::LogicalBlockSize::Lb4096;
    let disk = cfg
        .initialized(false)
        .logical_block_size(lb_size)
        .open(tempdisk.path())
        .unwrap();
    assert_eq!(*disk.logical_block_size(), lb_size);
    assert_eq!(disk.primary_header(), None);
    assert_eq!(disk.backup_header(), None);
    assert!(disk.partitions().is_empty());
}

#[test]
fn test_gptdisk_linux_01() {
    let diskpath = path::Path::new("tests/fixtures/gpt-linux-disk-01.img");
    let lb_size = disk::LogicalBlockSize::Lb512;

    let gdisk = gpt::GptConfig::new().open(diskpath).unwrap();
    assert_eq!(*gdisk.logical_block_size(), lb_size);
    assert!(gdisk.primary_header().is_some());
    assert!(gdisk.backup_header().is_some());
    assert_eq!(gdisk.partitions().len(), 1);

    let h1 = gdisk.primary_header().unwrap();
    assert_eq!(h1.current_lba, 1);
    assert_eq!(h1.backup_lba, 95);

    let h2 = gdisk.backup_header().unwrap();
    assert_eq!(h2.current_lba, 95);
    assert_eq!(h2.backup_lba, 1);

    let p1 = &gdisk.partitions().get(&1_u32).unwrap();
    assert_eq!(p1.name, "primary");
    let p1_start = p1.bytes_start(*gdisk.logical_block_size()).unwrap();
    assert_eq!(p1_start, 0x22 * 512);
    let p1_len = p1.bytes_len(*gdisk.logical_block_size()).unwrap();
    assert_eq!(p1_len, (0x3E - 0x22) * 512);
}

#[test]
fn test_gptdisk_linux_01_write_fidelity_with_device() {
    let diskpath = path::Path::new("tests/fixtures/gpt-linux-disk-01.img");

    // Assumes that test_gptdisk_linux_01 has passed, no need to check answers.
    let mut gdisk = gpt::GptConfig::new().open(diskpath).unwrap();
    let good_header1 = gdisk.primary_header().unwrap().clone();
    let good_header2 = gdisk.backup_header().unwrap().clone();
    let good_partitions = gdisk.partitions().clone();
    println!("good header1={:?}", good_header1);
    println!("good header2={:?}", good_header2);
    println!("good partitions={:#?}", good_partitions);

    // Test that we can write this test partition table to an in-memory buffer
    // instead, then load the results and verify they should be the same.
    let image_size = usize::try_from(std::fs::metadata(diskpath).unwrap().len()).unwrap();
    let mem_device = Box::new(std::io::Cursor::new(vec![0u8; image_size]));
    gdisk.update_disk_device(mem_device, true);
    let mut mem_device = gdisk.write().unwrap();

    // Write this memory buffer to a temp file, and load from the file to verify
    // that we wrote the data to the memory buffer correctly.
    let mut tempdisk = NamedTempFile::new().expect("failed to create tempfile disk");
    let mut gpt_in_mem = vec![0u8; image_size];
    let _ = mem_device.seek(SeekFrom::Start(0)).unwrap();
    mem_device.read_exact(&mut gpt_in_mem).unwrap();
    tempdisk.write_all(&gpt_in_mem).unwrap();
    tempdisk.flush().unwrap();

    let gdisk_file = gpt::GptConfig::new().open(tempdisk.path()).unwrap();
    println!("file header1={:?}", gdisk_file.primary_header().unwrap());
    println!("file header2={:?}", gdisk_file.backup_header().unwrap());
    println!("file partitions={:#?}", gdisk_file.partitions());
    assert_eq!(gdisk_file.primary_header().unwrap(), &good_header1);
    assert_eq!(gdisk_file.backup_header().unwrap(), &good_header2);
    assert_eq!(gdisk_file.partitions().clone(), good_partitions);

    // Test that if we read it back from this memory buffer, it matches the known good.
    let gdisk_mem = gpt::GptConfig::new().open_from_device(mem_device).unwrap();
    assert_eq!(gdisk_mem.primary_header().unwrap(), &good_header1);
    assert_eq!(gdisk_mem.backup_header().unwrap(), &good_header2);
    assert_eq!(gdisk_mem.partitions().clone(), good_partitions);
}
