use super::{Header, HeaderError};
use crate::disk::LogicalBlockSize;

use uuid::Uuid;

/// A builder struct to build a Header
#[derive(Debug, Clone)]
pub struct HeaderBuilder {
    primary: bool,
    disk_guid: Uuid,
    primary_lba: u64,
    backup_lba: u64,
    /// First usable LBA for partitions (primary table last LBA + 1)
    first_usable: u64,
    /// Last usable LBA (secondary partition table first LBA - 1)
    last_usable: u64,
    /// Number of partition entries
    num_parts: u32,
    /// Size of a partition entry, usually 128
    part_size: u32,
}

impl HeaderBuilder {
    /// Creates a new `HeaderBuilder`
    pub fn new() -> Self {
        Self {
            primary: true,
            disk_guid: Uuid::new_v4(),
            primary_lba: 1,
            backup_lba: 0,
            first_usable: 0,
            last_usable: 0,
            num_parts: super::MIN_NUM_PARTS,
            part_size: 128,
        }
    }

    /// Creates a new `HeaderBuilder` using the values from an existing Header
    pub fn from_header(header: &Header) -> Self {
        let primary = header.current_lba < header.backup_lba;

        let (primary_lba, backup_lba) = if primary {
            (header.current_lba, header.backup_lba)
        } else {
            (header.backup_lba, header.current_lba)
        };

        Self {
            primary,
            disk_guid: header.disk_guid,
            primary_lba,
            backup_lba,
            first_usable: header.first_usable,
            last_usable: header.last_usable,
            num_parts: header.num_parts,
            part_size: header.part_size,
        }
    }

    pub(crate) fn from_maybe_header(header: Option<&Header>) -> Self {
        header.map(Self::from_header).unwrap_or_else(Self::new)
    }

    /// Set wether this header is the primary or the backup.
    pub fn primary(&mut self, primary: bool) -> &mut Self {
        self.primary = primary;
        self
    }

    /// Change the disk guid, by default is generates a new one.
    pub fn disk_guid(&mut self, disk_guid: Uuid) -> &mut Self {
        self.disk_guid = disk_guid;
        self
    }

    /// Set the backup_lba position
    pub fn backup_lba(&mut self, backup_lba: u64) -> &mut Self {
        self.backup_lba = backup_lba;
        self
    }

    /// If you don't set this it will get calculated automatically
    ///
    /// It might still be calculated if the value is not sufficient
    pub fn first_usable(&mut self, first_usable: u64) -> &mut Self {
        self.first_usable = first_usable;
        self
    }

    /// If you don't set this it will get calculated automatically
    ///
    /// It might still be calculated if the value is not sufficient
    pub fn last_usable(&mut self, last_usable: u64) -> &mut Self {
        self.last_usable = last_usable;
        self
    }

    /// this will always set 128 >=
    ///
    /// If you already have partitions make sure num parts is bigger or equal
    ///
    /// ## Warning
    /// This might change the first usable and last usable part
    pub fn num_parts(&mut self, num_parts: u32) -> &mut Self {
        self.num_parts = num_parts.max(super::MIN_NUM_PARTS);
        self
    }

    /// This should probably be 128 but can be set lower
    ///
    /// ## Warning
    /// This might change the first usable and last usable part
    pub fn part_size(&mut self, part_size: u32) -> &mut Self {
        self.part_size = part_size;
        self
    }

    /// At least the following functions need to be called if the header
    /// doesn't get copied
    /// - backup_lba
    pub fn build(&mut self, lb_size: LogicalBlockSize) -> Result<Header, HeaderError> {
        // validate data
        if self.backup_lba < self.primary_lba {
            return Err(HeaderError::MissingBackupLba);
        }

        let (current_lba, backup_lba) = if self.primary {
            (self.primary_lba, self.backup_lba)
        } else {
            (self.backup_lba, self.primary_lba)
        };

        let part_array_size = self.num_parts * self.part_size;
        let part_array_lbs = u64_div_ceil(part_array_size as u64, lb_size.as_u64());

        let first_usable = self.first_usable.max(
            // mbr, header, part_array
            1 + 1 + part_array_lbs,
        );

        let last_usable = self.last_usable.max(
            // last is inclusive: end of disk is (partition array) (backup header)
            self.backup_lba
                .checked_sub(part_array_lbs + 1)
                .ok_or(HeaderError::BackupLbaToEarly)?,
        );

        if first_usable > last_usable {
            return Err(HeaderError::BackupLbaToEarly);
        }

        let part_start = if self.primary {
            self.primary_lba + 1
        } else {
            last_usable + 1
        };

        Ok(Header {
            signature: "EFI PART".to_string(),
            revision: (1, 0),
            header_size_le: 92,
            crc32: 0,
            reserved: 0,
            current_lba,
            backup_lba,
            first_usable,
            last_usable,
            disk_guid: self.disk_guid,
            part_start,
            num_parts: self.num_parts,
            part_size: self.part_size,
            crc32_parts: 0,
        })
    }
}

impl Default for HeaderBuilder {
    fn default() -> Self {
        Self::new()
    }
}

fn u64_div_ceil(lhs: u64, rhs: u64) -> u64 {
    (lhs + (rhs - 1)) / rhs
}
