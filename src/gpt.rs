use std::fs::File;
use std::io::{Read, Seek, Cursor};
use std::io::{SeekFrom, Error, ErrorKind};

extern crate uuid;
extern crate byteorder;
extern crate crc;

use self::byteorder::{LittleEndian, ReadBytesExt};
use self::uuid::Uuid;
use self::crc::crc32;

#[derive(Debug)]
pub struct Header {
    pub signature: String, // EFI PART
    pub revision: u32, // 00 00 01 00
    pub header_size_le: u32, // little endian
    pub crc32: u32,
    pub reserved: u32, // must be 0
    pub current_lba: u64,
    pub backup_lba: u64,
    pub first_usable: u64,
    pub last_usable: u64,
    pub disk_guid: uuid::Uuid,
    pub start_lba: u64,
    pub num_parts: u32,
    pub part_size: u32, // usually 128
    pub crc32_parts: u32,
}

fn parse_uuid(rdr: &mut Cursor<&[u8]>) -> Result<Uuid, Error> {
    //let mut rdr = Cursor::new(bytes);
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
        start_lba:      reader.read_u64::<LittleEndian>()?,
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