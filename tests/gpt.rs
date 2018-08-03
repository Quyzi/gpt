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
