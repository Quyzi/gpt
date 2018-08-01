//! Partition-related types and helper functions.

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
use self::itertools::Itertools;
use partition_types::PART_HASHMAP;
use uuid;

bitflags! {
    pub struct PartitionAttributes: u64 {
        const PLATFORM   = 0;
        const EFI        = 1;
        const BOOTABLE   = (1 << 1);
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct Partition {
    /// Contains the GUID of the type of partition.
    pub part_type_guid: PartitionType,
    /// UUID of the partition.
    pub part_guid: uuid::Uuid,
    /// First LBA of the partition
    pub first_lba: u64,
    /// Last LBA of the partition
    pub last_lba: u64,
    /// Partition flags
    pub flags: u64,
    /// Name of the partition (36 UTF-16LE characters)
    pub name: String,
}

impl Partition {
    fn as_bytes(&self) -> Result<Vec<u8>> {
        let mut buff: Vec<u8> = Vec::new();

        buff.write_all(self.part_type_guid.guid.as_bytes())?;
        buff.write_all(self.part_guid.as_bytes())?;
        buff.write_u64::<LittleEndian>(self.first_lba)?;
        buff.write_u64::<LittleEndian>(self.last_lba)?;
        buff.write_u64::<LittleEndian>(self.flags)?;
        buff.write_all(self.name.as_bytes())?;

        trace!("Partition Buffer: {:02x}", buff.iter().format(","));
        Ok(buff)
    }

    pub fn write(&self, p: &Path, h: &Header) -> Result<()> {
        // Write the partition to the part entry area
        // and rerun crc32 for the Header
        debug!("writing partition to file: {}", p.display());
        let mut file = OpenOptions::new().write(true).read(true).open(p)?;
        trace!("Seeking to {}", h.part_start * 512);
        file.seek(SeekFrom::Start(h.part_start * 512))?;
        file.write_all(&self.as_bytes()?)?;

        let parts_checksum = partentry_checksum(&mut file, disk::DEFAULT_SECTOR_SIZE)?;
        // Seek to partition checksum location and overwrite
        let _ = file.seek(SeekFrom::Start((h.current_lba * 512) + 88))?;
        file.write_u32::<LittleEndian>(parts_checksum)?;

        Ok(())
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct PartitionType {
    pub os: String,
    pub guid: String,
    pub desc: String,
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
            self.part_type_guid.desc,
            self.first_lba,
            self.last_lba,
            self.flags
        )
    }
}

fn read_part_name(rdr: &mut Cursor<&[u8]>) -> Result<String> {
    trace!("Reading partition name from {:?}", rdr);
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
    debug!("Parsing partition guid");
    let s = u.hyphenated().to_string().to_uppercase();
    debug!("Looking up partition type");
    match PART_HASHMAP.get(&s) {
        Some(part_id) => PartitionType {
            guid: s,
            os: part_id.0.into(),
            desc: part_id.1.into(),
        },
        None => {
            error!("Unknown partition type: {}", s);
            PartitionType {
                guid: s,
                os: "".to_string(),
                desc: "".to_string(),
            }
        }
    }
}

/// Read a gpt partition table.
///
/// let header = read_header("/dev/sda").unwrap();
/// let partitions: Vec<Partition> = read_partitions("/dev/sda", &mut header);
///
pub fn read_partitions(path: &str, header: &Header) -> Result<Vec<Partition>> {
    debug!("reading partitions from file: {}", path);
    let mut file = File::open(path)?;
    file_read_partitions(&mut file, header, disk::DEFAULT_SECTOR_SIZE.into())
}

/// Read a GPT partition table from an open `File` object.
pub(crate) fn file_read_partitions(
    file: &mut File,
    header: &Header,
    lb_size: u64,
) -> Result<Vec<Partition>> {
    trace!("Seeking to {}", lb_size * header.part_start);
    let _ = file.seek(SeekFrom::Start(lb_size * header.part_start));
    let mut parts: Vec<Partition> = Vec::new();

    debug!("Reading partitions");
    for _ in 0..header.num_parts {
        let mut bytes: [u8; 56] = [0; 56];
        let mut nameraw: [u8; 72] = [0; 72];

        let _ = file.read_exact(&mut bytes);
        let _ = file.read_exact(&mut nameraw);
        let partname = read_part_name(&mut Cursor::new(&nameraw[..]))?;

        let mut reader = Cursor::new(&bytes[..]);

        let p: Partition = Partition {
            part_type_guid: parse_parttype_guid(parse_uuid(&mut reader)?),
            part_guid: parse_uuid(&mut reader)?,
            first_lba: reader.read_u64::<LittleEndian>()?,
            last_lba: reader.read_u64::<LittleEndian>()?,
            flags: reader.read_u64::<LittleEndian>()?,
            name: partname.to_string(),
        };

        if p.part_guid.simple().to_string() != "00000000000000000000000000000000" {
            parts.push(p);
        }
    }

    trace!("Seeking to {}", lb_size * header.part_start);
    let _ = file.seek(SeekFrom::Start(lb_size * header.part_start));
    let mut table: [u8; 16384] = [0; 16384];
    let _ = file.read_exact(&mut table);

    debug!("Checking checksum");
    if crc32::checksum_ieee(&table) != header.crc32_parts {
        return Err(Error::new(ErrorKind::Other, "Invalid partition table CRC."));
    }

    Ok(parts)
}
