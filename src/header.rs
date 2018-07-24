use std::fs::{File, OpenOptions};
use std::io::{Cursor, SeekFrom, Error, ErrorKind, Read, Result, Seek, Write};
use std::fmt;
use std::path::Path;

extern crate byteorder;
extern crate crc;
extern crate itertools;
extern crate uuid;

use self::itertools::Itertools;

use self::byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use self::uuid::Uuid;
use self::crc::{crc32, Hasher32};

#[derive(Debug, Eq, PartialEq)]
pub struct Header {
    /// EFI PART
    pub signature: String, // Offset  0. "EFI PART", 45h 46h 49h 20h 50h 41h 52h 54h
    /// 00 00 01 00
    pub revision: u32, // Offset  8
    /// little endian
    pub header_size_le: u32, // Offset 12
    /// CRC32 of the header with crc32 section zeroed
    pub crc32: u32, // Offset 16
    /// must be 0
    pub reserved: u32, // Offset 20
    /// For main header, 1
    pub current_lba: u64, // Offset 24
    /// LBA for backup header
    pub backup_lba: u64, // Offset 32
    /// First usable LBA for partitions (primary table last LBA + 1)
    pub first_usable: u64, // Offset 40
    /// Last usable LBA (seconary partition table first LBA - 1)
    pub last_usable: u64, // Offset 48
    /// UUID of the disk
    pub disk_guid: uuid::Uuid, // Offset 56
    /// Starting LBA of partition entries
    pub part_start: u64, // Offset 72
    /// Number of partition entries
    pub num_parts: u32, // Offset 80
    /// Size of a partition entry, usually 128
    pub part_size: u32, // Offset 84
    /// CRC32 of the partition table
    pub crc32_parts: u32, // Offset 88
}

impl Header {
    /// Write the header to a location.  With a crc32 set to zero
    /// this will set the crc32 after writing the Header out
    fn write_primary(&self, file: &mut File) -> Result<usize> {
        let mut bytes_written = 0;
        let mbr = protective_mbr(file)?;

        //Write protective_mbr
        let _ = file.seek(SeekFrom::Start(448))?;
        bytes_written += file.write(&mbr)?;
        // signature word
        let _ = file.seek(SeekFrom::Start(510))?;
        bytes_written += file.write(&[0x55, 0xAA])?;

        // Build up byte array in memory
        let parts_checksum = partentry_checksum(file)?;
        let bytes = self.as_bytes(None, Some(parts_checksum))?;

        // Calculate the crc32 from the byte array
        let checksum = calculate_crc32(&bytes)?;

        let _ = file.seek(SeekFrom::Start(self.current_lba * 512))?;
        // Write it to disk in 1 shot
        bytes_written += file.write(
            &self.as_bytes(Some(checksum), Some(parts_checksum))?,
        )?;

        Ok(bytes_written)
    }

    fn write_backup(&self, file: &mut File) -> Result<usize> {
        file.seek(SeekFrom::End(self.backup_lba as i64))?;

        Ok(0)
    }

    fn as_bytes(&self, checksum: Option<u32>, parts_checksum: Option<u32>) -> Result<Vec<u8>> {
        let mut buff: Vec<u8> = Vec::new();

        buff.write(self.signature.as_bytes())?;
        buff.write_u32::<LittleEndian>(self.revision)?;
        buff.write_u32::<LittleEndian>(self.header_size_le)?;
        match checksum {
            Some(c) => buff.write_u32::<LittleEndian>(c)?,
            None => buff.write_u32::<LittleEndian>(0)?,
        };
        buff.write_u32::<LittleEndian>(0)?;
        buff.write_u64::<LittleEndian>(self.current_lba)?;
        buff.write_u64::<LittleEndian>(self.backup_lba)?;
        buff.write_u64::<LittleEndian>(self.first_usable)?;
        buff.write_u64::<LittleEndian>(self.last_usable)?;
        buff.write(self.disk_guid.as_bytes())?;
        buff.write_u64::<LittleEndian>(self.part_start)?;
        buff.write_u32::<LittleEndian>(self.num_parts)?;
        buff.write_u32::<LittleEndian>(self.part_size)?;
        match parts_checksum {
            Some(c) => buff.write_u32::<LittleEndian>(c)?,
            None => buff.write_u32::<LittleEndian>(0)?,
        };
        trace!("Buffer: {:02x}", buff.iter().format(","));
        Ok(buff)
    }
}

/// Parses a uuid with first 3 portions in little endian.
pub fn parse_uuid(rdr: &mut Cursor<&[u8]>) -> Result<Uuid> {
    let d1: u32 = rdr.read_u32::<LittleEndian>()?;
    let d2: u16 = rdr.read_u16::<LittleEndian>()?;
    let d3: u16 = rdr.read_u16::<LittleEndian>()?;
    let uuid = Uuid::from_fields(
        d1,
        d2,
        d3,
        &rdr.get_ref()[rdr.position() as usize..rdr.position() as usize + 8],
    );
    rdr.seek(SeekFrom::Current(8))?;

    match uuid {
        Ok(uuid) => Ok(uuid),
        Err(_) => Err(Error::new(ErrorKind::Other, "Invalid Disk UUID?")),
    }
}

impl fmt::Display for Header {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Disk:\t\t{}\nCRC32:\t\t{}\nTable CRC:\t{}",
            self.disk_guid,
            self.crc32,
            self.crc32_parts
        )
    }
}

/// Read a GPT header from a given path.
///
/// use gpt::header::read_header;
///
/// let h = read_header("/dev/sda")?;
///
pub fn read_header(path: &str) -> Result<Header> {
    let mut file = File::open(path)?;
    let _ = file.seek(SeekFrom::Start(512));

    let mut hdr: [u8; 92] = [0; 92];

    let _ = file.read_exact(&mut hdr);
    let mut reader = Cursor::new(&hdr[..]);

    let sigstr = String::from_utf8_lossy(
        &reader.get_ref()[reader.position() as usize..reader.position() as usize + 8],
    );
    reader.seek(SeekFrom::Current(8))?;

    if sigstr != "EFI PART" {
        return Err(Error::new(ErrorKind::Other, "Invalid GPT Signature."));
    };

    let h = Header {
        signature: sigstr.to_string(),
        revision: reader.read_u32::<LittleEndian>()?,
        header_size_le: reader.read_u32::<LittleEndian>()?,
        crc32: reader.read_u32::<LittleEndian>()?,
        reserved: reader.read_u32::<LittleEndian>()?,
        current_lba: reader.read_u64::<LittleEndian>()?,
        backup_lba: reader.read_u64::<LittleEndian>()?,
        first_usable: reader.read_u64::<LittleEndian>()?,
        last_usable: reader.read_u64::<LittleEndian>()?,
        disk_guid: parse_uuid(&mut reader)?,
        part_start: reader.read_u64::<LittleEndian>()?,
        num_parts: reader.read_u32::<LittleEndian>()?,
        part_size: reader.read_u32::<LittleEndian>()?,
        crc32_parts: reader.read_u32::<LittleEndian>()?,
    };

    let mut hdr_crc = hdr;
    for i in 16..20 {
        hdr_crc[i] = 0;
    }
    let c = crc32::checksum_ieee(&hdr_crc);
    trace!("hdr_crc: {:?}, h.crc32: {:?}", c, h.crc32);
    if crc32::checksum_ieee(&hdr_crc) == h.crc32 {
        Ok(h)
    } else {
        return Err(Error::new(ErrorKind::Other, "Invalid CRC32."));
    }
}

fn find_backup_lba(f: &mut File) -> Result<u64> {
    trace!("Querying file size to find backup header location");
    let m = f.metadata()?;
    let backup_location = (m.len() - 512) / 512;
    trace!("Backup location: {}", backup_location);

    Ok(backup_location)
}


fn calculate_crc32(b: &[u8]) -> Result<u32> {
    let mut digest = crc32::Digest::new(crc32::IEEE);
    trace!("Writing buffer to digest calculator");
    digest.write(b);

    Ok(digest.sum32())
}

pub(crate) fn partentry_checksum(file: &mut File) -> Result<u32> {
    // Seek to LBA 2
    let _ = file.seek(SeekFrom::Start(2 * 512))?;
    let mut buff: [u8; 65536] = [0; 65536];
    file.read_exact(&mut buff)?;

    let mut digest = crc32::Digest::new(crc32::IEEE);
    digest.write(&buff);

    Ok(digest.sum32())
}

fn protective_mbr(f: &mut File) -> Result<Vec<u8>> {
    let m = f.metadata()?;
    let len = m.len() / 512;
    let mut buff: Vec<u8> = Vec::new();

    //Boot Indicator. Must set to 00 so the partition can't be booted
    buff.write_u8(0)?;
    buff.write_u8(0)?; // starting head
    buff.write_u8(0)?; // starting sector
    buff.write_u8(0)?; // starting cylinder
    buff.write_u8(0xEE)?; // System ID.  Must be EE for GPT
    //Ending Head. Same as Ending LBA of the single partition
    if len > 255 {
        buff.write_u8(0xFF)?;
        //Ending Sector. Same as Ending LBA of the single partition
        buff.write_u8(0xFF)?;
        //Ending Cylinder. Same as Ending LBA of the single partition
        buff.write_u8(0xFF)?;
    } else {
        buff.write_u8(len as u8)?;
        buff.write_u8(len as u8)?;
        buff.write_u8(len as u8)?;
    }
    //Starting LBA. Always set to 1.
    buff.write_u32::<LittleEndian>(1)?;
    //Size in LBA. The size of the partition. Set to FF FF FF FF if too large
    if len as u32 > u32::max_value() {
        buff.write_u32::<LittleEndian>(u32::max_value())?;
    } else {
        buff.write_u32::<LittleEndian>(len as u32)?;
    }

    Ok(buff)
}

/// A helper function to create a new header and write it to disk.
/// If the uuid isn't given a random one will be generated.  Use
/// this in conjunction with Partition::write()
// TODO: Move this to Header::new() and Header::write to write it
// that will match the Partition::write() API
pub fn write_header(p: &Path, uuid: Option<Uuid>) -> Result<Uuid> {
    let return_uuid: Uuid;
    debug!("opening {} for writing", p.display());
    let mut file = OpenOptions::new().write(true).read(true).open(p)?;
    let backup_location = find_backup_lba(&mut file)?;

    if let Some(disk_guid) = uuid {
        return_uuid = disk_guid;
    } else {
        debug!("Generating random uuid");
        return_uuid = Uuid::new_v4();
    }

    let h = Header {
        signature: "EFI PART".to_string(),
        revision: 65536,
        header_size_le: 92,
        crc32: 0,
        reserved: 0,
        current_lba: 1,
        // LBA -1
        backup_lba: backup_location,
        first_usable: 34,
        // LBA -34
        last_usable: backup_location - 33,
        disk_guid: return_uuid,
        part_start: 2,
        num_parts: 128,
        part_size: 128,
        crc32_parts: 0,
    };
    debug!("Header: {:#?}", h);
    h.write_primary(&mut file)?;

    Ok(return_uuid)
}
