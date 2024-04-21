//! MBR-related types and helper functions.
//!
//! This module provides access to low-level primitives
//! to work with Master Boot Record (MBR), also known as LBA0.

use crate::disk;
use crate::DiskDevice;
use std::{fmt, io};

use simple_bytes::{Bytes, BytesArray, BytesRead, BytesWrite};

#[non_exhaustive]
#[derive(Debug)]
/// Errors returned when interacting with a Gpt Disk.
pub enum MBRError {
    /// Generic IO Error
    Io(io::Error),
    /// The provided buffer does not match the expected mbr length
    InvalidMBRLength,
    /// invalid MBR signature
    InvalidMBRSignature,
    /// Invalid Partition Length != 16
    InvalidPartitionLength,
    /// Somthing Overflowed or Underflowed
    /// This will never occur when dealing with sane values
    Overflow(&'static str),
}

impl From<io::Error> for MBRError {
    fn from(e: io::Error) -> Self {
        Self::Io(e)
    }
}

impl std::error::Error for MBRError {}

impl fmt::Display for MBRError {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        use MBRError::*;
        let desc = match self {
            Io(e) => return write!(fmt, "MBR IO Error: {e}"),
            InvalidMBRLength => "The provided buffer does not match the expected mbr length",
            InvalidMBRSignature => "Invalid MBR signature",
            InvalidPartitionLength => "Invalid Partition length expected 16",
            Overflow(m) => return write!(fmt, "MBR error Overflow: {m}"),
        };
        write!(fmt, "{desc}")
    }
}

const MBR_SIGNATURE: [u8; 2] = [0x55, 0xAA];

/// Protective MBR, as defined by GPT.
pub struct ProtectiveMBR {
    bootcode: [u8; 440],
    disk_signature: [u8; 4],
    unknown: u16,
    partitions: [PartRecord; 4],
    signature: [u8; 2],
}

impl fmt::Debug for ProtectiveMBR {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Protective MBR, partitions: {:#?}", self.partitions)
    }
}

impl Default for ProtectiveMBR {
    fn default() -> Self {
        Self {
            bootcode: [0x00; 440],
            disk_signature: [0x00; 4],
            unknown: 0,
            partitions: [
                PartRecord::new_protective(None),
                PartRecord::zero(),
                PartRecord::zero(),
                PartRecord::zero(),
            ],
            signature: MBR_SIGNATURE,
        }
    }
}

impl ProtectiveMBR {
    /// Create a default protective-MBR object.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a protective-MBR object with a specific protective partition size (in LB).
    /// The protective partition size should be the size of the disk - 1 (because the protective
    /// partition always begins at LBA 1 (the second sector)).
    pub fn with_lb_size(lb_size: u32) -> Self {
        Self {
            bootcode: [0x00; 440],
            disk_signature: [0x00; 4],
            unknown: 0,
            partitions: [
                PartRecord::new_protective(Some(lb_size)),
                PartRecord::zero(),
                PartRecord::zero(),
                PartRecord::zero(),
            ],
            signature: MBR_SIGNATURE,
        }
    }

    /// Parse input bytes into a protective-MBR object.
    pub fn from_bytes(buf: &[u8], sector_size: disk::LogicalBlockSize) -> Result<Self, MBRError> {
        let mut pmbr = Self::new();
        let totlen: u64 = sector_size.into();

        if buf.len() != (totlen as usize) {
            return Err(MBRError::InvalidMBRLength);
        }

        let mut bytes = Bytes::from(buf);

        pmbr.bootcode.copy_from_slice(bytes.read(440));
        pmbr.disk_signature.copy_from_slice(bytes.read(4));
        pmbr.unknown = bytes.read_le_u16();

        for p in pmbr.partitions.iter_mut() {
            *p = PartRecord::from_bytes(bytes.read(16))?;
        }

        assert_eq!(simple_bytes::BytesSeek::position(&bytes), 510);

        pmbr.signature.copy_from_slice(bytes.read(2));
        if pmbr.signature == MBR_SIGNATURE {
            Ok(pmbr)
        } else {
            Err(MBRError::InvalidMBRSignature)
        }
    }

    /// Read the LBA0 of a disk device and parse it into a protective-MBR object.
    pub fn from_disk<D: DiskDevice>(
        device: &mut D,
        sector_size: disk::LogicalBlockSize,
    ) -> Result<Self, MBRError> {
        let totlen: u64 = sector_size.into();
        let mut buf = vec![0_u8; totlen as usize];
        let cur = device.stream_position()?;

        device.seek(io::SeekFrom::Start(0))?;
        device.read_exact(&mut buf)?;
        let pmbr = Self::from_bytes(&buf, sector_size);
        device.seek(io::SeekFrom::Start(cur))?;
        pmbr
    }

    /// Return the memory representation of this MBR as a byte vector.
    ///
    /// This will always be 512
    pub fn to_bytes(&self) -> [u8; 512] {
        let mut bytes = BytesArray::from([0u8; 512]);

        bytes.write(self.bootcode);
        bytes.write(self.disk_signature);
        bytes.write_le_u16(self.unknown);

        for p in &self.partitions {
            bytes.write(p.to_bytes());
        }

        bytes.write(self.signature);

        bytes.into_array()
    }

    /// Return the 440 bytes of BIOS bootcode.
    pub fn bootcode(&self) -> &[u8; 440] {
        &self.bootcode
    }

    /// Set the 440 bytes of BIOS bootcode.
    ///
    /// This only changes the in-memory state, without overwriting
    /// any on-disk data.
    pub fn set_bootcode(&mut self, bootcode: [u8; 440]) -> &Self {
        self.bootcode = bootcode;
        self
    }

    /// Return the 4 bytes of MBR disk signature.
    pub fn disk_signature(&self) -> &[u8; 4] {
        &self.disk_signature
    }

    /// Set the 4 bytes of MBR disk signature.
    ///
    /// This only changes the in-memory state, without overwriting
    /// any on-disk data.
    pub fn set_disk_signature(&mut self, sig: [u8; 4]) -> &Self {
        self.disk_signature = sig;
        self
    }

    /// Returns the given partition (0..=3) or None if the partition index is invalid.
    pub fn partition(&self, partition_index: usize) -> Option<PartRecord> {
        if partition_index >= self.partitions.len() {
            None
        } else {
            Some(self.partitions[partition_index])
        }
    }

    /// Set the data for the given partition.
    /// Returns the previous partition record or None if the partition index is invalid.
    ///
    /// This only changes the in-memory state, without overwriting
    /// any on-disk data.
    pub fn set_partition(
        &mut self,
        partition_index: usize,
        partition: PartRecord,
    ) -> Option<PartRecord> {
        if partition_index >= self.partitions.len() {
            None
        } else {
            Some(std::mem::replace(
                &mut self.partitions[partition_index],
                partition,
            ))
        }
    }

    /// Write a protective MBR to LBA0, overwriting any existing data.
    ///
    /// This will not write the entire lba0 if the sector size is 4096
    pub fn overwrite_lba0<D: DiskDevice>(&self, device: &mut D) -> Result<usize, MBRError> {
        let cur = device.stream_position()?;
        let _ = device.seek(io::SeekFrom::Start(0))?;
        let data = self.to_bytes();
        device.write_all(&data)?;
        device.flush()?;

        device.seek(io::SeekFrom::Start(cur))?;
        Ok(data.len())
    }

    /// Update LBA0, preserving most bytes of any existing MBR.
    ///
    /// This overwrites the four MBR partition records and the
    /// well-known signature, leaving all other MBR bits as-is.
    pub fn update_conservative<D: DiskDevice>(&self, device: &mut D) -> Result<usize, MBRError> {
        let cur = device.stream_position()?;
        // Seek to first partition record.
        // (GPT spec 2.7 - sec. 5.2.3 - table 15)
        let _ = device.seek(io::SeekFrom::Start(446))?;
        for p in &self.partitions {
            device.write_all(&p.to_bytes())?;
        }
        device.write_all(&self.signature)?;
        device.flush()?;

        device.seek(io::SeekFrom::Start(cur))?;
        let bytes_updated: usize = (16 * 4) + 2;
        Ok(bytes_updated)
    }
}

/// A partition record, MBR-style.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct PartRecord {
    /// Bit 7 set if partition is active (bootable)
    pub boot_indicator: u8,
    /// CHS address of partition start: 8-bit value of head in CHS address
    pub start_head: u8,
    /// CHS address of partition start: Upper 2 bits are 8th-9th bits of cylinder, lower 6 bits are sector
    pub start_sector: u8,
    /// CHS address of partition start: Lower 8 bits of cylinder
    pub start_track: u8,
    /// Partition type. See <https://www.win.tue.nl/~aeb/partitions/partition_types-1.html>
    pub os_type: u8,
    /// CHS address of partition end: 8-bit value of head in CHS address
    pub end_head: u8,
    /// CHS address of partition end: Upper 2 bits are 8th-9th bits of cylinder, lower 6 bits are sector
    pub end_sector: u8,
    /// CHS address of partition end: Lower 8 bits of cylinder
    pub end_track: u8,
    /// LBA of start of partition
    pub lb_start: u32,
    /// Number of sectors in partition
    pub lb_size: u32,
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

    /// Parse input bytes into a Partition Record.
    pub fn from_bytes(buf: &[u8]) -> Result<Self, MBRError> {
        if buf.len() != 16 {
            return Err(MBRError::InvalidPartitionLength);
        }

        let mut bytes = Bytes::from(buf);

        let pr = Self {
            boot_indicator: bytes.read_u8(),
            start_head: bytes.read_u8(),
            start_sector: bytes.read_u8(),
            start_track: bytes.read_u8(),
            os_type: bytes.read_u8(),
            end_head: bytes.read_u8(),
            end_sector: bytes.read_u8(),
            end_track: bytes.read_u8(),
            lb_start: bytes.read_le_u32(),
            lb_size: bytes.read_le_u32(),
        };

        Ok(pr)
    }

    /// Return the memory representation of this Partition Record as a byte vector.
    pub fn to_bytes(&self) -> [u8; 16] {
        let mut bytes = BytesArray::from([0u8; 16]);

        bytes.write_u8(self.boot_indicator);

        bytes.write_u8(self.start_head);
        bytes.write_u8(self.start_sector);
        bytes.write_u8(self.start_track);

        bytes.write_u8(self.os_type);

        bytes.write_u8(self.end_head);
        bytes.write_u8(self.end_sector);
        bytes.write_u8(self.end_track);

        bytes.write_le_u32(self.lb_start);
        bytes.write_le_u32(self.lb_size);

        bytes.into_array()
    }
}

/// Return the 440 bytes of BIOS bootcode.
pub fn read_bootcode<D: DiskDevice>(device: &mut D) -> io::Result<[u8; 440]> {
    let bootcode_offset = 0;
    let cur = device.stream_position()?;
    let _ = device.seek(io::SeekFrom::Start(bootcode_offset))?;
    let mut bootcode = [0x00; 440];
    device.read_exact(&mut bootcode)?;

    device.seek(io::SeekFrom::Start(cur))?;
    Ok(bootcode)
}

/// Write the 440 bytes of BIOS bootcode.
pub fn write_bootcode<D: DiskDevice>(device: &mut D, bootcode: &[u8; 440]) -> io::Result<()> {
    let bootcode_offset = 0;
    let cur = device.stream_position()?;
    let _ = device.seek(io::SeekFrom::Start(bootcode_offset))?;
    device.write_all(bootcode)?;
    device.flush()?;

    device.seek(io::SeekFrom::Start(cur))?;
    Ok(())
}

/// Read the 4 bytes of MBR disk signature.
pub fn read_disk_signature<D: DiskDevice>(device: &mut D) -> io::Result<[u8; 4]> {
    let dsig_offset = 440;
    let cur = device.stream_position()?;
    let _ = device.seek(io::SeekFrom::Start(dsig_offset))?;
    let mut dsig = [0x00; 4];
    device.read_exact(&mut dsig)?;

    device.seek(io::SeekFrom::Start(cur))?;
    Ok(dsig)
}

/// Write the 4 bytes of MBR disk signature.
#[cfg_attr(feature = "cargo-clippy", allow(clippy::trivially_copy_pass_by_ref))]
pub fn write_disk_signature<D: DiskDevice>(device: &mut D, sig: &[u8; 4]) -> io::Result<()> {
    let dsig_offset = 440;
    let cur = device.stream_position()?;
    let _ = device.seek(io::SeekFrom::Start(dsig_offset))?;
    device.write_all(sig)?;
    device.flush()?;

    device.seek(io::SeekFrom::Start(cur))?;
    Ok(())
}
