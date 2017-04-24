use std::fs::File;
use std::io::{Read, Seek, Cursor};
use std::io::{SeekFrom, Error, ErrorKind};
use std::fmt; 

extern crate uuid;
extern crate byteorder;
extern crate crc;

use self::byteorder::{LittleEndian, ReadBytesExt};
use self::uuid::Uuid;
use self::crc::crc32;

#[derive(Debug)]
pub struct Header {
    /// EFI PART
    pub signature: String, 
    /// 00 00 01 00
    pub revision: u32, 
    /// little endian
    pub header_size_le: u32, 
    /// CRC32 of the header with crc32 section zeroed
    pub crc32: u32, 
    /// must be 0
    pub reserved: u32, 
    /// For main header, 1
    pub current_lba: u64, 
    /// LBA for backup header
    pub backup_lba: u64, 
    /// First usable LBA for partitions (primary table last LBA + 1)
    pub first_usable: u64, 
    /// Last usable LBA (seconary partition table first LBA - 1)
    pub last_usable: u64, 
    /// UUID of the disk
    pub disk_guid: uuid::Uuid, 
    /// Starting LBA of partition entries
    pub part_start: u64, 
    /// Number of partition entries
    pub num_parts: u32, 
    /// Size of a partition entry, usually 128
    pub part_size: u32, 
    /// CRC32 of the partition table
    pub crc32_parts: u32, 

}

/// Parses a uuid with first 3 portions in little endian. 
pub fn parse_uuid(rdr: &mut Cursor<&[u8]>) -> Result<Uuid, Error> {
    let d1: u32 = rdr.read_u32::<LittleEndian>()?;
    let d2: u16 = rdr.read_u16::<LittleEndian>()?;
    let d3: u16 = rdr.read_u16::<LittleEndian>()?;
    let uuid = Uuid::from_fields(d1, d2, d3, &rdr.get_ref()[rdr.position() as usize .. rdr.position() as usize + 8]);
    rdr.seek(SeekFrom::Current(8))?;

    match uuid {
        Ok(uuid) => Ok(uuid),
        Err(_) => Err(Error::new(ErrorKind::Other, "Invalid Disk UUID?")),
    }
}

impl fmt::Display for Header {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Disk:\t\t{}\nCRC32:\t\t{}\nTable CRC:\t{}", self.disk_guid, self.crc32, self.crc32_parts)
    }
}

/// Read a GPT header from a given path. 
///
/// use gpt::header::read_header;
///
/// let h = read_header("/dev/sda")?;
///
pub fn read_header(path: &String) -> Result<Header, Error> {
    let mut file = File::open(path)?;
    let _ = file.seek(SeekFrom::Start(512));

    let mut hdr: [u8; 92] = [0; 92];

    let _ = file.read_exact(&mut hdr);
    let mut reader = Cursor::new(&hdr[..]);

    let sigstr = String::from_utf8_lossy(&reader.get_ref()[reader.position() as usize .. reader.position() as usize + 8]);
    reader.seek(SeekFrom::Current(8))?;

    if sigstr != "EFI PART" {
        return Err(Error::new(ErrorKind::Other, "Invalid GPT Signature."));
    };

    let h = Header {
        signature:      sigstr.to_string(),
        revision:       reader.read_u32::<LittleEndian>()?,
        header_size_le: reader.read_u32::<LittleEndian>()?,
        crc32:          reader.read_u32::<LittleEndian>()?,
        reserved:       reader.read_u32::<LittleEndian>()?,
        current_lba:    reader.read_u64::<LittleEndian>()?,
        backup_lba:     reader.read_u64::<LittleEndian>()?,
        first_usable:   reader.read_u64::<LittleEndian>()?,
        last_usable:    reader.read_u64::<LittleEndian>()?,
        disk_guid:      parse_uuid(&mut reader)?,
        part_start:      reader.read_u64::<LittleEndian>()?,
        num_parts:      reader.read_u32::<LittleEndian>()?,
        part_size:      reader.read_u32::<LittleEndian>()?,
        crc32_parts:    reader.read_u32::<LittleEndian>()?,
    };

    let mut hdr_crc = hdr;
    for i in 16..20
    {
    	hdr_crc[i] = 0;
    }
    if crc32::checksum_ieee(&hdr_crc) == h.crc32
    {
    	Ok(h)
    }
    else 
    {
        return Err(Error::new(ErrorKind::Other, "Invalid CRC32."))
    }
}