//! Disk-related types and helper functions.

use super::{GptConfig, GptDisk};
use std::{io, path};

/// Default size of a logical sector (bytes).
pub const DEFAULT_SECTOR_SIZE: LogicalBlockSize = LogicalBlockSize::Lb512;

/// Logical block/sector size of a GPT disk.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LogicalBlockSize {
    /// 512 bytes.
    Lb512,
    /// 4096 bytes.
    Lb4096,
}

impl Into<u64> for LogicalBlockSize {
    fn into(self) -> u64 {
        match self {
            LogicalBlockSize::Lb512 => 512,
            LogicalBlockSize::Lb4096 => 4096,
        }
    }
}

/// Open and read a GPT disk, using default configuration options.
pub fn read_disk(diskpath: &path::Path) -> io::Result<GptDisk> {
    let cfg = GptConfig::new();
    cfg.open(diskpath)
}
