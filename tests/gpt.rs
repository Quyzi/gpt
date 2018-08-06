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

    let disk = gpt::GptConfig::new().open(diskpath).unwrap();
    assert_eq!(*disk.logical_block_size(), lb_size);
    assert!(disk.primary_header().is_some());
    assert!(disk.backup_header().is_some());
    assert_eq!(disk.partitions().len(), 1);

    let h1 = disk.primary_header().unwrap();
    assert_eq!(h1.current_lba, 1);
    assert_eq!(h1.backup_lba, 95);

    let h2 = disk.backup_header().unwrap();
    assert_eq!(h2.current_lba, 95);
    assert_eq!(h2.backup_lba, 1);

    let p1 = &disk.partitions()[0];
    assert_eq!(p1.name, "primary");
    assert_eq!(p1.part_type_guid.desc, "Linux Filesystem Data");
}
