//! Partition-related types and helper functions.
//!
//! This module provides access to low-level primitives
//! to work with GPT partitions.

use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use crc::crc32;
use std::fmt;
use std::fs::{File, OpenOptions};
use std::io::{Cursor, Error, ErrorKind, Read, Result, Seek, SeekFrom, Write};
use std::path::Path;
use uuid;

use disk;
use header::{parse_uuid, partentry_checksum, Header};
use partition_types::PART_HASHMAP;

bitflags! {
    /// Partition entry attributes, defined for UEFI.
    pub struct PartitionAttributes: u64 {
        /// Required platform partition.
        const PLATFORM   = 1;
        /// No Block-IO protocol.
        const EFI        = (1 << 1);
        /// Legacy-BIOS bootable partition.
        const BOOTABLE   = (1 << 2);
    }
}

/// A partition entry in a GPT partition table.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Partition {
    /// GUID of the partition type.
    pub part_type_guid: PartitionType,
    /// UUID of the partition.
    pub part_guid: uuid::Uuid,
    /// First LBA of the partition.
    pub first_lba: u64,
    /// Last LBA of the partition.
    pub last_lba: u64,
    /// Partition flags.
    pub flags: u64,
    /// Partition name.
    pub name: String,
}

impl Partition {
    /// Create a partition entry of type "unused", whose bytes are all 0s.
    pub fn zero() -> Self {
        Self {
            part_type_guid: PartitionType {
                guid: uuid::Uuid::nil(),
                os: "".to_string(),
                description: "".to_string(),
            },
            part_guid: uuid::Uuid::nil(),
            first_lba: 0,
            last_lba: 0,
            flags: 0,
            name: "".to_string(),
        }
    }

    /// Serialize this partition entry to its bytes representation.
    fn as_bytes(&self, entry_size: u16) -> Result<Vec<u8>> {
        let mut buf: Vec<u8> = Vec::with_capacity(entry_size as usize);

        // Type GUID.
        let tyguid = self.part_type_guid.guid.as_fields();
        buf.write_u32::<LittleEndian>(tyguid.0)?;
        buf.write_u16::<LittleEndian>(tyguid.1)?;
        buf.write_u16::<LittleEndian>(tyguid.2)?;
        buf.write_all(tyguid.3)?;

        // Partition GUID.
        let pguid = self.part_guid.as_fields();
        buf.write_u32::<LittleEndian>(pguid.0)?;
        buf.write_u16::<LittleEndian>(pguid.1)?;
        buf.write_u16::<LittleEndian>(pguid.2)?;
        buf.write_all(pguid.3)?;

        // LBAs and flags.
        buf.write_u64::<LittleEndian>(self.first_lba)?;
        buf.write_u64::<LittleEndian>(self.last_lba)?;
        buf.write_u64::<LittleEndian>(self.flags)?;

        // Partition name as UTF16-LE.
        for utf16_char in self.name.encode_utf16().take(36) {
            buf.write_u16::<LittleEndian>(utf16_char)?;
        }

        // Resize buffer to exact entry size.
        buf.resize(entry_size as usize, 0x00);

        Ok(buf)
    }

    /// Write the partition entry to the partitions area and update crc32 for the Header.
    pub fn write(&self, p: &Path, h: &Header, lb_size: disk::LogicalBlockSize) -> Result<()> {
        debug!("writing partition to file: {}", p.display());
        let pstart = h.part_start
            .checked_mul(lb_size.into())
            .ok_or_else(|| Error::new(ErrorKind::Other, "partition overflow - start offset"))?;
        let mut file = OpenOptions::new().write(true).read(true).open(p)?;
        trace!("seeking to partition start: {:#x}", pstart);
        file.seek(SeekFrom::Start(pstart))?;
        file.write_all(&self.as_bytes(128)?)?;

        let parts_checksum = partentry_checksum(&mut file, h, lb_size)?;
        // Seek to header partition checksum location and update it.
        let hdr_csum = h.current_lba
            .checked_mul(lb_size.into())
            .ok_or_else(|| Error::new(ErrorKind::Other, "partition overflow - header start"))?
            .checked_add(88)
            .ok_or_else(|| Error::new(ErrorKind::Other, "partition overflow - checksum offset"))?;
        let _ = file.seek(SeekFrom::Start(hdr_csum))?;
        file.write_u32::<LittleEndian>(parts_checksum)?;

        Ok(())
    }

    /// Return the length (in bytes) of this partition.
    pub fn bytes_len(&self, lb_size: disk::LogicalBlockSize) -> Result<u64> {
        let len = self.last_lba
            .checked_sub(self.first_lba)
            .ok_or_else(|| Error::new(ErrorKind::Other, "partition length underflow - sectors"))?
            .checked_mul(lb_size.into())
            .ok_or_else(|| Error::new(ErrorKind::Other, "partition length overflow - bytes"))?;
        Ok(len)
    }

    /// Return the starting offset (in bytes) of this partition.
    pub fn bytes_start(&self, lb_size: disk::LogicalBlockSize) -> Result<u64> {
        let len = self.first_lba
            .checked_mul(lb_size.into())
            .ok_or_else(|| Error::new(ErrorKind::Other, "partition start overflow - bytes"))?;
        Ok(len)
    }
}

/// Partition type, with optional description.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PartitionType {
    /// Type-GUID for a GPT partition.
    pub guid: uuid::Uuid,
    /// Optional well-known OS label for this type-GUID.
    pub os: String,
    /// Optional well-known description label for this type-GUID.
    pub description: String,
}

impl fmt::Display for Partition {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Partition:\t\t{}\nPartition GUID:\t\t{}\nPartition Type:\t\t{}\t{}\n\
             Span:\t\t\t{} - {}\nFlags:\t\t\t{}",
            self.name,
            self.part_guid,
            self.part_type_guid.guid,
            self.part_type_guid.description,
            self.first_lba,
            self.last_lba,
            self.flags
        )
    }
}

fn read_part_name(rdr: &mut Cursor<&[u8]>) -> Result<String> {
    trace!("Reading partition name");
    let mut namebytes: Vec<u16> = Vec::new();
    for _ in 0..36 {
        let b = rdr.read_u16::<LittleEndian>()?;
        if b != 0 {
            namebytes.push(b);
        }
    }

    Ok(String::from_utf16_lossy(&namebytes))
}

fn parse_parttype_guid(u: uuid::Uuid) -> PartitionType {
    let s = u.to_hyphenated().to_string().to_uppercase();
    debug!("looking up partition type, GUID {}", s);
    match PART_HASHMAP.get(&s) {
        Some(part_id) => PartitionType {
            guid: u,
            os: part_id.0.into(),
            description: part_id.1.into(),
        },
        None => {
            error!("Unknown partition type: {}", s);
            PartitionType {
                guid: u,
                os: "".to_string(),
                description: "".to_string(),
            }
        }
    }
}

/// Read a GPT partition table.
///
/// ## Example
///
/// ```rust,no_run
/// use gpt::{header, disk, partition};
/// use std::path::Path;
///
/// let lb_size = disk::DEFAULT_SECTOR_SIZE;
/// let diskpath = Path::new("/dev/sdz");
/// let hdr = header::read_header(diskpath, lb_size).unwrap();
/// let partitions = partition::read_partitions(diskpath, &hdr, lb_size).unwrap();
/// println!("{:#?}", partitions);
/// ```
pub fn read_partitions(
    path: &Path,
    header: &Header,
    lb_size: disk::LogicalBlockSize,
) -> Result<Vec<Partition>> {
    debug!("reading partitions from file: {}", path.display());
    let mut file = File::open(path)?;
    file_read_partitions(&mut file, header, lb_size)
}

/// Read a GPT partition table from an open `File` object.
pub(crate) fn file_read_partitions(
    file: &mut File,
    header: &Header,
    lb_size: disk::LogicalBlockSize,
) -> Result<Vec<Partition>> {
    let pstart = header
        .part_start
        .checked_mul(lb_size.into())
        .ok_or_else(|| Error::new(ErrorKind::Other, "partition overflow - start offset"))?;
    trace!("seeking to partitions start: {:#x}", pstart);
    let _ = file.seek(SeekFrom::Start(pstart))?;
    let mut parts: Vec<Partition> = Vec::new();

    trace!("scanning {} partitions", header.num_parts);
    for _ in 0..header.num_parts {
        let mut bytes: [u8; 56] = [0; 56];
        let mut nameraw: [u8; 72] = [0; 72];

        file.read_exact(&mut bytes)?;
        file.read_exact(&mut nameraw)?;

        let mut reader = Cursor::new(&bytes[..]);
        let type_guid = parse_uuid(&mut reader)?;
        let part_guid = parse_uuid(&mut reader)?;

        if part_guid.to_simple().to_string() == "00000000000000000000000000000000" {
            continue;
        }

        let partname = read_part_name(&mut Cursor::new(&nameraw[..]))?;
        let p: Partition = Partition {
            part_type_guid: parse_parttype_guid(type_guid),
            part_guid,
            first_lba: reader.read_u64::<LittleEndian>()?,
            last_lba: reader.read_u64::<LittleEndian>()?,
            flags: reader.read_u64::<LittleEndian>()?,
            name: partname.to_string(),
        };

        parts.push(p);
    }

    debug!("checking partition table CRC");
    let _ = file.seek(SeekFrom::Start(pstart))?;
    let pt_len = u64::from(header.num_parts)
        .checked_mul(header.part_size.into())
        .ok_or_else(|| Error::new(ErrorKind::Other, "partitions - size"))?;
    let mut table = vec![0; pt_len as usize];
    file.read_exact(&mut table)?;

    let comp_crc = crc32::checksum_ieee(&table);
    if comp_crc != header.crc32_parts {
        return Err(Error::new(ErrorKind::Other, "partition table CRC mismatch"));
    }

    Ok(parts)
}

#[cfg(test)]
mod tests {
    use disk;
    use partition;

    #[test]
    fn test_zero_part() {
        let p0 = partition::Partition::zero();

        let b128 = p0.as_bytes(128).unwrap();
        assert_eq!(b128.len(), 128);
        assert_eq!(b128, vec![0u8; 128]);

        let b256 = p0.as_bytes(256).unwrap();
        assert_eq!(b256.len(), 256);
        assert_eq!(b256, vec![0u8; 256]);
    }

    #[test]
    fn test_part_bytes_len() {
        {
            // Zero.
            let p0 = partition::Partition::zero();
            let b512len = p0.bytes_len(disk::LogicalBlockSize::Lb512).unwrap();
            let b4096len = p0.bytes_len(disk::LogicalBlockSize::Lb4096).unwrap();

            assert_eq!(b512len, 0);
            assert_eq!(b4096len, 0);
        }

        {
            // Negative length.
            let mut p1 = partition::Partition::zero();
            p1.first_lba = p1.last_lba + 1;
            p1.bytes_len(disk::LogicalBlockSize::Lb512).unwrap_err();
            p1.bytes_len(disk::LogicalBlockSize::Lb4096).unwrap_err();
        }

        {
            // Overflowing u64 length.
            let mut p2 = partition::Partition::zero();
            p2.last_lba = <u64>::max_value();
            p2.bytes_len(disk::LogicalBlockSize::Lb512).unwrap_err();
            p2.bytes_len(disk::LogicalBlockSize::Lb4096).unwrap_err();
        }

        {
            // Positive value.
            let mut p3 = partition::Partition::zero();
            p3.first_lba = 2;
            p3.last_lba = 4;
            let b512len = p3.bytes_len(disk::LogicalBlockSize::Lb512).unwrap();
            let b4096len = p3.bytes_len(disk::LogicalBlockSize::Lb4096).unwrap();

            assert_eq!(b512len, 2 * 512);
            assert_eq!(b4096len, 2 * 4096);
        }
    }

    #[test]
    fn test_part_bytes_start() {
        {
            // Zero.
            let p0 = partition::Partition::zero();
            let b512len = p0.bytes_start(disk::LogicalBlockSize::Lb512).unwrap();
            let b4096len = p0.bytes_start(disk::LogicalBlockSize::Lb4096).unwrap();

            assert_eq!(b512len, 0);
            assert_eq!(b4096len, 0);
        }

        {
            // Overflowing u64 start.
            let mut p1 = partition::Partition::zero();
            p1.first_lba = <u64>::max_value();
            p1.bytes_len(disk::LogicalBlockSize::Lb512).unwrap_err();
            p1.bytes_len(disk::LogicalBlockSize::Lb4096).unwrap_err();
        }

        {
            // Positive value.
            let mut p2 = partition::Partition::zero();
            p2.first_lba = 2;
            let b512start = p2.bytes_start(disk::LogicalBlockSize::Lb512).unwrap();
            let b4096start = p2.bytes_start(disk::LogicalBlockSize::Lb4096).unwrap();

            assert_eq!(b512start, 2 * 512);
            assert_eq!(b4096start, 2 * 4096);
        }
    }
}
