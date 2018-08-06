//! Partition-related types and helper functions.
//!
//! This module provides access to low-level primitives
//! to work with GPT partitions.

use std::fmt;
use std::fs::{File, OpenOptions};
use std::io::{Cursor, Error, ErrorKind, Read, Result, Seek, SeekFrom, Write};
use std::path::Path;

use disk;
use header::{parse_uuid, partentry_checksum, Header};

extern crate byteorder;
extern crate crc;
extern crate itertools;

use self::byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use self::crc::crc32;
use partition_types::PART_HASHMAP;
use uuid;

bitflags! {
    /// Partition entry attributes, defined for UEFI.
    pub struct PartitionAttributes: u64 {
        const PLATFORM   = 0;
        const EFI        = 1;
        const BOOTABLE   = (1 << 1);
    }
}

/// A partition entry in a GPT partition table.
#[derive(Debug, Eq, PartialEq)]
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
}

/// Partition type, with optional description.
#[derive(Debug, Eq, PartialEq)]
pub struct PartitionType {
    pub guid: uuid::Uuid,
    pub os: String,
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
    let s = u.hyphenated().to_string().to_uppercase();
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

        if part_guid.simple().to_string() == "00000000000000000000000000000000" {
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
