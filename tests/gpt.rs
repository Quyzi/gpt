extern crate gpt;

use gpt::disk;
use std::path;

#[test]
fn test_gptconfig() {
    let devnull = path::Path::new("/dev/null");
    let cfg = {
        let c1 = gpt::GptConfig::new();
        let c2 = gpt::GptConfig::default();
        assert_eq!(c1, c2);
        c1
    };

    let lb_size = disk::LogicalBlockSize::Lb4096;
    let disk = cfg.initialized(false)
        .logical_block_size(lb_size)
        .open(devnull)
        .unwrap();
    assert_eq!(*disk.logical_block_size(), lb_size);
    assert_eq!(*disk.primary_header(), None);
    assert_eq!(*disk.backup_header(), None);
    assert!(disk.partitions().is_empty());
}
