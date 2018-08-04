//! MBR-related types and helper functions.
//!
//! This module provides access to low-level primitives
//! to work with Master Boot Record (MBR), also known as LBA0.

use byteorder::{LittleEndian, WriteBytesExt};
use std::io::{Seek, Write};
use std::{fmt, fs, io};

/// Protective MBR, as defined by GPT.
pub struct ProtectiveMBR {
    bootcode: [u8; 440],
    disk_signature: u32,
    unknown: u16,
    partitions: [PartRecord; 4],
    signature: [u8; 2],
}

impl fmt::Debug for ProtectiveMBR {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Protective MBR, partitions: {:#?}", self.partitions)
    }
}

impl Default for ProtectiveMBR {
    fn default() -> Self {
        Self {
            bootcode: [0x00; 440],
            disk_signature: 0,
            unknown: 0,
            partitions: [
                PartRecord::new_protective(None),
                PartRecord::zero(),
                PartRecord::zero(),
                PartRecord::zero(),
            ],
            signature: [0x55, 0xAA],
        }
    }
}

impl ProtectiveMBR {
    /// Create a default protective-MBR object.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a protective-MBR object with a specific disk size (in LB).
    pub fn with_lb_size(lb_size: u32) -> Self {
        Self {
            bootcode: [0x00; 440],
            disk_signature: 0,
            unknown: 0,
            partitions: [
                PartRecord::new_protective(Some(lb_size)),
                PartRecord::zero(),
                PartRecord::zero(),
                PartRecord::zero(),
            ],
            signature: [0x55, 0xAA],
        }
    }

    /// Return the memory representation of this MBR as a byte vector.
    pub fn as_bytes(&self) -> io::Result<Vec<u8>> {
        let mut buf: Vec<u8> = Vec::with_capacity(512);

        buf.write_all(&self.bootcode)?;
        buf.write_u32::<LittleEndian>(self.disk_signature)?;
        buf.write_u16::<LittleEndian>(self.unknown)?;
        for p in &self.partitions {
            let pdata = p.as_bytes()?;
            buf.write_all(&pdata)?;
        }
        buf.write_all(&self.signature)?;
        Ok(buf)
    }

    /// Write a protective MBR to LBA0, overwriting any existing data.
    pub fn overwrite_lba0(&self, file: &mut fs::File) -> io::Result<usize> {
        let cur = file.seek(io::SeekFrom::Current(0))?;
        let _ = file.seek(io::SeekFrom::Start(0))?;
        let data = self.as_bytes()?;
        file.write_all(&data)?;

        file.seek(io::SeekFrom::Start(cur))?;
        Ok(data.len())
    }

    /// Update LBA0, preserving most bytes of any existing MBR.
    ///
    /// This overwrites the four MBR partition records and the
    /// well-known signature, leaving all other MBR bits as-is.
    pub fn update_conservative(&self, file: &mut fs::File) -> io::Result<usize> {
        let cur = file.seek(io::SeekFrom::Current(0))?;
        // Seek to first partition record.
        // (GPT spec 2.7 - sec. 5.2.3 - table 15)
        let _ = file.seek(io::SeekFrom::Start(446))?;
        for p in &self.partitions {
            let pdata = p.as_bytes()?;
            file.write_all(&pdata)?;
        }
        file.write_all(&self.signature)?;

        file.seek(io::SeekFrom::Start(cur))?;
        let bytes_updated: usize = (16 * 4) + 2;
        Ok(bytes_updated)
    }
}

/// A partition record, MBR-style.
#[derive(Debug, Eq, PartialEq)]
pub struct PartRecord {
    boot_indicator: u8,
    start_head: u8,
    start_sector: u8,
    start_track: u8,
    os_type: u8,
    end_head: u8,
    end_sector: u8,
    end_track: u8,
    lb_start: u32,
    lb_size: u32,
}

impl PartRecord {
    /// Create a protective Partition Record object with a specific disk size (in LB).
    pub fn new_protective(lb_size: Option<u32>) -> Self {
        let size = lb_size.unwrap_or(0xFF_FF_FF_FF);
        Self {
            boot_indicator: 0x00,
            start_head: 0x00,
            start_sector: 0x02,
            start_track: 0x00,
            os_type: 0xEE,
            end_head: 0xFF,
            end_sector: 0xFF,
            end_track: 0xFF,
            lb_start: 1,
            lb_size: size,
        }
    }

    /// Create an all-zero Partition Record.
    pub fn zero() -> Self {
        Self {
            boot_indicator: 0x00,
            start_head: 0x00,
            start_sector: 0x00,
            start_track: 0x00,
            os_type: 0x00,
            end_head: 0x00,
            end_sector: 0x00,
            end_track: 0x00,
            lb_start: 0,
            lb_size: 0,
        }
    }

    /// Return the memory representation of this Partition Record as a byte vector.
    pub fn as_bytes(&self) -> io::Result<Vec<u8>> {
        let mut buf: Vec<u8> = Vec::with_capacity(16);

        buf.write_u8(self.boot_indicator)?;

        buf.write_u8(self.start_head)?;
        buf.write_u8(self.start_sector)?;
        buf.write_u8(self.start_track)?;

        buf.write_u8(self.os_type)?;

        buf.write_u8(self.end_head)?;
        buf.write_u8(self.end_sector)?;
        buf.write_u8(self.end_track)?;

        buf.write_u32::<LittleEndian>(self.lb_start)?;
        buf.write_u32::<LittleEndian>(self.lb_size)?;

        Ok(buf)
    }
}
