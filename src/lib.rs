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
//!
//! fn create_partition() {
//!     let diskpath = std::path::Path::new("/tmp/chris.img");
//!     let cfg = gpt::GptConfig::new().writable(true).initialized(true);
//!     let mut disk = cfg.open(diskpath).expect("failed to open disk");
//!     let result = disk.add_partition(
//!         "rust_partition",
//!         100,
//!         gpt::partition_types::LINUX_FS,
//!         0,
//!     );
//!     disk.write().unwrap();
//! }
//! ```

#![deny(missing_docs)]

use log::*;
use std::collections::BTreeMap;
use std::io::Write;
use std::{fs, io, path};

#[macro_use]
mod macros;
pub mod disk;
pub mod header;
pub mod mbr;
pub mod partition;
pub mod partition_types;

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
                partitions: BTreeMap::new(),
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
    partitions: BTreeMap<u32, partition::Partition>,
}

impl GptDisk {
    /// Sort the partition IDs by partition order and make sure they
    /// start at one.  If this is not done, fdisk will complain that
    /// the partitions are out of order, and it also causes problems
    /// with other tools.
    pub fn normalize(&mut self) {
	let keys : Vec<u32> = self.partitions.iter().map(|(&id,_)| id).collect();
	let mut sorted_partitions = BTreeMap::new();
	for k in keys.iter() {
	    if let Some(part) = self.partitions.remove(k) {
		sorted_partitions.insert(part.first_lba,part);
	    }
	}
	let mut id = 0;
	for (_,part) in sorted_partitions.into_iter() {
	    id += 1;
	    self.partitions.insert(id,part);
	}
    }

    /// Add another partition to this disk.  This tries to find
    /// the optimum partition location with the lowest block device.
    /// Returns the new partition id if there was sufficient room
    /// to add the partition. Size is specified in bytes.
    pub fn add_partition(
        &mut self,
        name: &str,
        size: u64,
        part_type: partition_types::Type,
        flags: u64,
    ) -> io::Result<u32> {
        let size_lba = match size.checked_div(self.config.lb_size.into()) {
            Some(s) => s,
            None => {
                return Err(io::Error::new(
                    io::ErrorKind::Other,
                    format!(
                        "size must be greater than {} which is the logical block size.",
                        self.config.lb_size
                    ),
                ));
            }
        };
        // Find the lowest lba that is larger than size.
        let free_sections = self.find_free_sectors();
        for (starting_lba, length) in free_sections {
            debug!("starting_lba {}, length {}", starting_lba, length);
            if length >= (size_lba - 1) {
                // Found our free slice.
                let partition_id = self.find_next_partition_id();
                debug!(
                    "Adding partition id: {} {:?}.  first_lba: {} last_lba: {}",
                    partition_id,
                    part_type,
                    starting_lba,
                    starting_lba + size_lba - 1 as u64
                );
                let part = partition::Partition {
                    part_type_guid: part_type,
                    part_guid: uuid::Uuid::new_v4(),
                    first_lba: starting_lba,
                    last_lba: starting_lba + size_lba - 1 as u64,
                    flags,
                    name: name.to_string(),
                };
                if let Some(p) = self.partitions.insert(partition_id, part.clone()) {
                    debug!("Replacing\n{}\nwith\n{}", p, part);
                }
                return Ok(partition_id);
            }
        }

        Err(io::Error::new(
            io::ErrorKind::Other,
            "Unable to find enough space on drive",
        ))
    }
    /// remove partition from this disk. This tries to find the partition based on either a
    /// given partition number (id) or a partition guid.  Returns the partition id if the
    /// partition is removed
    pub fn remove_partition(
        &mut self,
        id: Option<u32>,
        partguid: Option<uuid::Uuid>,
    ) -> io::Result<u32> {
        if let Some(part_id) = id {
            if let Some(partition_id) = self.partitions.remove(&part_id) {
                debug!("Removing partition number {}", partition_id);
            }
            return Ok(part_id);
        }
        if let Some(part_guid) = partguid {
            for (key, partition) in &self.partitions.clone() {
                if partition.part_guid == part_guid {
                    if let Some(partition_id) = self.partitions.remove(&key) {
                        debug!("Removing partition number {}", partition_id);
                    }
                    return Ok(*key);
                }
            }
        }
        Err(io::Error::new(
            io::ErrorKind::Other,
            "Unable to find partition to remove",
        ))
    }

    /// Find free space on the disk.
    /// Returns a tuple of (starting_lba, length in lba's).
    pub fn find_free_sectors(&self) -> Vec<(u64, u64)> {
        if let Some(header) = self.primary_header().or_else(|| self.backup_header()) {
            trace!("first_usable: {}", header.first_usable);
            let mut disk_positions = vec![header.first_usable];
            for part in self.partitions().iter().filter(|p| p.1.is_used()) {
                trace!("partition: ({}, {})", part.1.first_lba, part.1.last_lba);
                disk_positions.push(part.1.first_lba);
                disk_positions.push(part.1.last_lba);
            }
            disk_positions.push(header.last_usable);
            trace!("last_usable: {}", header.last_usable);
            disk_positions.sort();

            return disk_positions
                // Walk through the LBA's in chunks of 2 (ending, starting).
                .chunks(2)
                // Add 1 to the ending and then subtract the starting if NOT the first usable sector
                .map(|p| {
                    if p[0] != header.first_usable {
                        (p[0] + 1, p[1].saturating_sub(p[0] + 1))
                    } else {
                        (p[0], p[1].saturating_sub(p[0]))
                    }
                })
                .collect();
        }
        // No primary header. Return nothing.
        vec![]
    }

    /// Find next highest partition id.
    pub fn find_next_partition_id(&self) -> u32 {
        let max = match self
            .partitions()
            .iter()
            // Skip unused partitions.
            .filter(|p| p.1.is_used())
            // Find the maximum id.
            .max_by_key(|x| x.0)
        {
            Some(i) => i.0 + 0,
            // Partitions start at 1.
            None => return 1,
        };
        for i in 1..max {
            if self.partitions().get(&i).is_none() {
                return i;
            }
        }
        max + 1
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
    pub fn partitions(&self) -> &BTreeMap<u32, partition::Partition> {
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
    pub fn update_partitions(
        &mut self,
        pp: BTreeMap<u32, partition::Partition>,
    ) -> io::Result<&Self> {
        // TODO(lucab): validate partitions.
        let bak = header::find_backup_lba(&mut self.file, self.config.lb_size)?;
        let h1 = header::Header::compute_new(true, &pp, self.guid, bak, &self.primary_header)?;
        let h2 = header::Header::compute_new(false, &pp, self.guid, bak, &self.backup_header)?;
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
	let num_defined_parts = self.partitions().len() as u32;
        trace!("Number of partitions to write: {}",num_defined_parts);
        let zerop = partition::Partition::zero();
        let num_parts = header::GPT_MIN_NUM_PARTITION_ENTRIES.max(num_defined_parts);
        // The partition IDs in the GptDisk.partitions tree start at one
        for i in 1..=num_parts {
            let partition =
                match self.partitions.get(&i) {
                    None => &zerop,
                    Some(p) => &p
                };
            partition.write(
                &self.path,
                (i - 1) as u64,
                self.primary_header.clone().unwrap().part_start,
                self.config.lb_size,
            )?;
        }
        let new_backup_header = header::Header::compute_new(
            false,
            &self.partitions,
            self.guid,
            bak,
            &self.primary_header,
        )?;
        let new_primary_header = header::Header::compute_new(
            true,
            &self.partitions,
            self.guid,
            bak,
            &self.backup_header,
        )?;
        debug!("Writing backup header");
        new_backup_header.write_backup(&mut self.file, self.config.lb_size)?;
        debug!("Writing primary header");
        new_primary_header.write_primary(&mut self.file, self.config.lb_size)?;
        trace!("new primary header: {:?}", new_primary_header);
        trace!("new backup header: {:?}", new_backup_header);

        self.file.flush()?;
        self.primary_header = Some(new_primary_header.clone());
        self.backup_header = Some(new_backup_header);

        Ok(self.file)
    }
}
