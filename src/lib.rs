//! A pure-Rust library to work with GPT partition tables.
//!
//! It provides support for manipulating (R/W) GPT headers and partition
//! tables. Raw disk devices as well as disk images are supported.
//!
//! ```rust,no_run
//! extern crate gpt;
//!
//! fn inspect_disk() {
//!     let diskpath = std::path::Path::new("/dev/sdz");
//!     let cfg = gpt::GptConfig::new().writable(false);
//!
//!     let disk = cfg.open(diskpath).expect("failed to open disk");
//!
//!     println!("Disk header: {:#?}", disk.primary_header());
//!     println!("Partition layout: {:#?}", disk.partitions());
//! }
//! ```

#![deny(missing_docs)]

use bitflags;
use lazy_static;
use log::*;
use std::cmp::Ordering;
use std::io::Write;
use std::{fs, io, path};

pub mod disk;
pub mod header;
pub mod mbr;
pub mod partition;
mod partition_types;

/// Configuration options to open a GPT disk.
#[derive(Debug, Eq, PartialEq)]
pub struct GptConfig {
    /// Logical block size.
    lb_size: disk::LogicalBlockSize,
    /// Whether to open a GPT partition table in writable mode.
    writable: bool,
    /// Whether to expect and parse an initialized disk image.
    initialized: bool,
}

impl GptConfig {
    // TODO(lucab): complete support for skipping backup
    // header, etc, then expose all config knobs here.

    /// Create a new default configuration.
    pub fn new() -> Self {
        GptConfig::default()
    }

    /// Whether to open a GPT partition table in writable mode.
    pub fn writable(mut self, writable: bool) -> Self {
        self.writable = writable;
        self
    }

    /// Whether to assume an initialized GPT disk and read its
    /// partition table on open.
    pub fn initialized(mut self, initialized: bool) -> Self {
        self.initialized = initialized;
        self
    }

    /// Size of logical blocks (sectors) for this disk.
    pub fn logical_block_size(mut self, lb_size: disk::LogicalBlockSize) -> Self {
        self.lb_size = lb_size;
        self
    }

    /// Open the GPT disk at the given path and inspect it according
    /// to configuration options.
    pub fn open(self, diskpath: &path::Path) -> io::Result<GptDisk> {
        // Uninitialized disk, no headers/table to parse.
        if !self.initialized {
            let file = fs::OpenOptions::new()
                .write(self.writable)
                .read(true)
                .open(diskpath)?;
            let empty = GptDisk {
                config: self,
                file,
                guid: uuid::Uuid::new_v4(),
                path: diskpath.to_path_buf(),
                primary_header: None,
                backup_header: None,
                partitions: vec![],
            };
            return Ok(empty);
        }

        // Proper GPT disk, fully inspect its layout.
        let mut file = fs::OpenOptions::new()
            .write(self.writable)
            .read(true)
            .open(diskpath)?;
        let h1 = header::read_primary_header(&mut file, self.lb_size)?;
        let h2 = header::read_backup_header(&mut file, self.lb_size)?;
        let table = partition::file_read_partitions(&mut file, &h1, self.lb_size)?;
        let disk = GptDisk {
            config: self,
            file,
            guid: h1.disk_guid,
            path: diskpath.to_path_buf(),
            primary_header: Some(h1),
            backup_header: Some(h2),
            partitions: table,
        };
        debug!("disk: {:?}", disk);
        Ok(disk)
    }
}

impl Default for GptConfig {
    fn default() -> Self {
        Self {
            lb_size: disk::DEFAULT_SECTOR_SIZE,
            initialized: true,
            writable: false,
        }
    }
}

/// A file-backed GPT disk.
#[derive(Debug)]
pub struct GptDisk {
    config: GptConfig,
    file: fs::File,
    guid: uuid::Uuid,
    path: path::PathBuf,
    primary_header: Option<header::Header>,
    backup_header: Option<header::Header>,
    partitions: Vec<partition::Partition>,
}

impl GptDisk {
    /// Add another partition to this disk.  This tries to find
    /// the optimum partition location with the lowest block device.
    pub fn add_partition(
        &mut self,
        name: &str,
        size: usize,
        part_type: partition::PartitionType,
        flags: u64,
    ) -> io::Result<()> {
        self.sort_partitions();
        // Find the lowest lba that is larger than size.
        let free_sections = self.find_free_sectors();
        for (starting_lba, length) in free_sections {
            if length as usize >= size {
                // Found our free slice.
                let partition_id = self.find_next_partition_id();
                debug!(
                    "Adding partition id: {} {:?}.  first_lba: {} last_lba: {}",
                    partition_id,
                    part_type,
                    starting_lba,
                    starting_lba + size as u64
                );
                let part = partition::Partition {
                    part_type_guid: part_type,
                    part_guid: uuid::Uuid::new_v4(),
                    first_lba: starting_lba,
                    last_lba: starting_lba + size as u64,
                    flags,
                    name: name.to_string(),
                    id: partition_id,
                };
                self.partitions.push(part);
                return Ok(());
            }
        }

        Err(io::Error::new(
            io::ErrorKind::Other,
            "Unable to find enough space on drive",
        ))
    }

    /// Find free space on the disk.
    /// Returns a tuple of (starting_lba, length in lba's).
    pub fn find_free_sectors(&self) -> Vec<(u64, u64)> {
        if let Some(primary_header) = self.primary_header() {
            debug!("first_usable: {}", primary_header.first_usable);
            let mut disk_positions = vec![primary_header.first_usable + 1];
            for part in self.partitions().iter().filter(|p| p.is_used()) {
                debug!("partition: ({}, {})", part.first_lba, part.last_lba);
                disk_positions.push(part.first_lba);
                disk_positions.push(part.last_lba);
            }
            disk_positions.push(primary_header.last_usable - 1);
            debug!("last_usable: {}", primary_header.last_usable);
            disk_positions.sort();

            return disk_positions
                // Walk through the LBA's in chunks of 2 (ending, starting).
                .chunks(2)
                // Add 1 to the ending and then subtract the starting.
                .map(|p| (p[0] + 1, p[1].saturating_sub(p[0])))
                .collect();
        }
        // No primary header. Return nothing.
        vec![]
    }

    /// Find next highest partition id.
    pub fn find_next_partition_id(&self) -> u32 {
        match self.partitions().iter().max_by(|x, y| x.id.cmp(&y.id)) {
            Some(i) => i.id + 1,
            None => 0,
        }
    }

    /// Retrieve primary header, if any.
    pub fn primary_header(&self) -> Option<&header::Header> {
        self.primary_header.as_ref()
    }

    /// Retrieve backup header, if any.
    pub fn backup_header(&self) -> Option<&header::Header> {
        self.backup_header.as_ref()
    }

    /// Retrieve partition entries.
    pub fn partitions(&self) -> &[partition::Partition] {
        &self.partitions
    }

    /// Retrieve disk UUID.
    pub fn guid(&self) -> &uuid::Uuid {
        &self.guid
    }

    /// Retrieve disk logical block size.
    pub fn logical_block_size(&self) -> &disk::LogicalBlockSize {
        &self.config.lb_size
    }

    /// Sort the partitions by their starting LBA.  Takes into
    /// account unused partitions.
    pub fn sort_partitions(&mut self) {
        self.partitions
            .sort_by(|a, b| match (a.is_used(), b.is_used()) {
                (true, true) => a.first_lba.cmp(&b.first_lba),
                (true, false) => Ordering::Less,
                (false, true) => Ordering::Greater,
                (false, false) => Ordering::Equal,
            });
    }

    /// Update disk UUID.
    ///
    /// If no UUID is specified, a new random one is generated.
    /// No changes are recorded to disk until `write()` is called.
    pub fn update_guid(&mut self, uuid: Option<uuid::Uuid>) -> io::Result<&Self> {
        let guid = match uuid {
            Some(u) => u,
            None => {
                let u = uuid::Uuid::new_v4();
                debug!("Generated random uuid: {}", u);
                u
            }
        };
        self.guid = guid;
        Ok(self)
    }

    /// Update current partition table.
    ///
    /// No changes are recorded to disk until `write()` is called.
    pub fn update_partitions(&mut self, pp: Vec<partition::Partition>) -> io::Result<&Self> {
        // TODO(lucab): validate partitions.
        let bak = header::find_backup_lba(&mut self.file, self.config.lb_size)?;
        let h1 = header::Header::compute_new(true, &pp, self.guid, bak)?;
        let h2 = header::Header::compute_new(false, &pp, self.guid, bak)?;
        self.primary_header = Some(h1);
        self.backup_header = Some(h2);
        self.partitions = pp;
        self.config.initialized = true;
        Ok(self)
    }

    /// Persist state to disk, consuming this disk object.
    ///
    /// This is a destructive action, as it overwrite headers and
    /// partitions entries on disk. All writes are flushed to disk
    /// before returning the underlying `File` object.
    pub fn write(mut self) -> io::Result<fs::File> {
        if !self.config.writable {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                "disk not opened in writable mode",
            ));
        }
        if !self.config.initialized {
            return Err(io::Error::new(io::ErrorKind::Other, "disk not initialized"));
        }
        debug!("Computing new headers");
        trace!("old primary header: {:?}", self.primary_header);
        trace!("old backup header: {:?}", self.backup_header);
        let bak = header::find_backup_lba(&mut self.file, self.config.lb_size)?;
        trace!("old backup lba: {}", bak);
        let new_backup_header =
            header::Header::compute_new(false, &self.partitions, self.guid, bak)?;
        let new_primary_header =
            header::Header::compute_new(true, &self.partitions, self.guid, bak)?;
        debug!("Writing backup header");
        new_backup_header.write_backup(&mut self.file, self.config.lb_size)?;
        debug!("Writing primary header");
        new_primary_header.write_primary(&mut self.file, self.config.lb_size)?;
        trace!("new primary header: {:?}", new_primary_header);
        trace!("new backup header: {:?}", new_backup_header);

        self.file.flush()?;
        self.primary_header = Some(new_primary_header.clone());
        self.backup_header = Some(new_backup_header);

        // Sort so we're not seeking all over the place.
        self.sort_partitions();
        for partition in self.partitions() {
            partition.write(&self.path, &new_primary_header, self.config.lb_size)?;
        }

        Ok(self.file)
    }
}
