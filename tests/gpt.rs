extern crate gpt;
extern crate tempfile;

use gpt::disk;
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
    let disk = cfg.initialized(false)
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

    let p1 = &gdisk.partitions()[0];
    assert_eq!(p1.name, "primary");
    assert_eq!(p1.part_type_guid.description, "Linux Filesystem Data");
    let p1_start = p1.bytes_start(*gdisk.logical_block_size()).unwrap();
    assert_eq!(p1_start, 0x22 * 512);
    let p1_len = p1.bytes_len(*gdisk.logical_block_size()).unwrap();
    assert_eq!(p1_len, (0x3E - 0x22) * 512);
}
