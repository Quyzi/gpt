use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, Cursor, SeekFrom, Error, ErrorKind, Result, Write};
use std::fmt;
use std::path::Path;

use header::{Header, parse_uuid, partentry_checksum};

extern crate byteorder;
extern crate crc;
extern crate itertools;
extern crate uuid;

use partition_types::PART_HASHMAP;
use self::byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use self::crc::crc32;
use self::itertools::Itertools;

bitflags! {
    pub struct PartitionAttributes: u64 {
        const PLATFORM   = 0;
        const EFI        = (1 << 0);
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
    pub first_LBA: u64,
    /// Last LBA of the partition
    pub last_LBA: u64,
    /// Partition flags
    pub flags: u64,
    /// Name of the partition (36 UTF-16LE characters)
    pub name: String,
}

impl Partition {
    fn as_bytes(&self) -> Result<Vec<u8>> {
        let mut buff: Vec<u8> = Vec::new();

        buff.write(self.part_type_guid.guid.as_bytes())?;
        buff.write(self.part_guid.as_bytes())?;
        buff.write_u64::<LittleEndian>(self.first_LBA)?;
        buff.write_u64::<LittleEndian>(self.last_LBA)?;
        buff.write_u64::<LittleEndian>(self.flags)?;
        buff.write(self.name.as_bytes())?;

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
        file.write(&self.as_bytes()?)?;

        let parts_checksum = partentry_checksum(&mut file)?;
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
            self.first_LBA,
            self.last_LBA,
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

    return Ok(String::from_utf16_lossy(&namebytes));
}

fn parse_parttype_guid(str: uuid::Uuid) -> PartitionType {
    debug!("Parsing partition guid");
    let uuid = str.hyphenated().to_string().to_uppercase();
    debug!("Looking up partition type");
    match PART_HASHMAP.get(&uuid) {
        Some(part_id) => {
            PartitionType {
                guid: uuid,
                os: part_id.0.into(),
                desc: part_id.1.into(),
            }
        }
        None => {
            error!("Unknown partition type: {}", uuid);
            PartitionType {
                guid: uuid,
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
    trace!("Seeking to {}", 512 * header.part_start);
    let _ = file.seek(SeekFrom::Start(512 * header.part_start));
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
            first_LBA: reader.read_u64::<LittleEndian>()?,
            last_LBA: reader.read_u64::<LittleEndian>()?,
            flags: reader.read_u64::<LittleEndian>()?,
            name: partname.to_string(),
        };

        if p.part_guid.simple().to_string() != "00000000000000000000000000000000" {
            parts.push(p);
        }
    }

    trace!("Seeking to {}", 512 * header.part_start);
    let _ = file.seek(SeekFrom::Start(512 * header.part_start));
    let mut table: [u8; 16384] = [0; 16384];
    let _ = file.read_exact(&mut table);

    debug!("Checking checksum");
    if crc32::checksum_ieee(&table) != header.crc32_parts {
        return Err(Error::new(ErrorKind::Other, "Invalid partition table CRC."));
    }

    Ok(parts)
}
