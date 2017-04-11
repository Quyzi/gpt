use std::fs::File;
use std::io::{Read, Seek, Cursor};
use std::io::{SeekFrom, Error, ErrorKind};
use std::convert::AsMut;

extern crate uuid;
extern crate byteorder;
use self::byteorder::{LittleEndian, ReadBytesExt, BigEndian};
use self::uuid::Uuid;
#[derive(Debug)]
pub struct Header
{
	pub signature: String, // EFI PART
	pub revision: u32, // 00 00 01 00
	pub header_size_le: [u8;4], // little endian
	pub crc32: [u8;4], 
	pub reserved: [u8;4], // must be 0
	pub current_lba: [u8; 8],
	pub backup_lba: [u8; 8],
	pub first_usable: [u8; 8],
	pub last_usable: [u8; 8],
	pub disk_guid: uuid::Uuid,
	pub start_lba: [u8; 8],
	pub num_parts: [u8; 4],
	pub part_size: [u8; 4], // usually 128
	pub crc32_parts: [u8; 4]
}

fn parse_uuid(bytes: &[u8]) -> Result<Uuid, Error>
{
	let mut rdr = Cursor::new(bytes);
	let d1: u32 = rdr.read_u32::<LittleEndian>().unwrap();
	let d2: u16 = rdr.read_u16::<LittleEndian>().unwrap();
	let d3: u16 = rdr.read_u16::<LittleEndian>().unwrap();

	match Uuid::from_fields(d1, d2, d3, &bytes[8..])
	{
		Ok(uuid) => Ok(uuid),
		Err(_) => Err(Error::new(ErrorKind::Other, "Invalid Disk UUID?"))
	}
}

fn clone_into_array<A, T>(slice: &[T]) -> A
    where A: Sized + Default + AsMut<[T]>,
          T: Clone
{
    let mut a = Default::default();
    <A as AsMut<[T]>>::as_mut(&mut a).clone_from_slice(slice);
    a
}

pub fn read_header2(path:&String) -> Result<Header, Error>
{
	let mut file = File::open(path)?;
	let _ = file.seek(SeekFrom::Start(512));

	let mut hdr: [u8; 92] = [0; 92];

	//let _ = file.read_exact(&mut hdr);
	let mut reader = Cursor::new(file);

	let sigstr: String = String::from_utf8_lossy(reader.read_u64::<LittleEndian>());

	if sigstr.as_ref() != "EFI PART" 
	{
		return Err(Error::new(ErrorKind::Other, "Invalid GPT Signature."))
	};


	let guid: uuid::Uuid = parse_uuid(&hdr[56..72])?;
	let h = Header{
		signature: sigstr.to_string(), //clone_into_array(&hdr[0..8]), 
		revision: &hdr[8..12], 
		header_size_le: clone_into_array(&hdr[12..16]), 
		crc32: clone_into_array(&hdr[16..20]), 
		reserved: clone_into_array(&hdr[20..24]),
		current_lba: clone_into_array(&hdr[24..32]),
		backup_lba: clone_into_array(&hdr[32..40]),
		first_usable: clone_into_array(&hdr[40..48]),
		last_usable: clone_into_array(&hdr[48..56]),
		disk_guid: guid,
		start_lba: clone_into_array(&hdr[72..80]),
		num_parts: clone_into_array(&hdr[80..84]),
		part_size: clone_into_array(&hdr[84..88]),
		crc32_parts: clone_into_array(&hdr[88..])
	};

	Ok(h)
}
/*
pub fn read_header(path:&String) -> Result<Header, Error>
{
	let mut file = File::open(path)?;
	let _ = file.seek(SeekFrom::Start(512))?;

	let mut signature: [u8;8] = [0;8];
	let _ = file.read_exact(&mut signature);
	let sigstr = String::from_utf8_lossy(&signature);
	if sigstr.as_ref() != "EFI PART" { return Err(Error::new(ErrorKind::Other, "Invalid GPT Signature")) };

	let mut revision: [u8; 4] = [0; 4];
	let mut header_size_le: [u8;4] = [0; 4];
	let mut crc32: [u8;4] = [0; 4];
	let mut reserved: [u8;4] = [0; 4];
	let mut current_lba: [u8;8] = [0; 8];
	let mut backup_lba: [u8;8] = [0; 8];
	let mut first_usable: [u8;8] = [0; 8];
	let mut last_usable: [u8;8] = [0; 8];
	let mut disk_guid: [u8;16] = [0; 16];
	let mut start_lba: [u8; 8] = [0; 8];
	let mut num_parts: [u8; 4] = [0; 4];
	let mut part_size: [u8; 4]= [0; 4];
	let mut crc32_parts: [u8; 4] = [0; 4];

	let _ = file.read_exact(&mut revision);
	let _ = file.read_exact(&mut header_size_le);
	let _ = file.read_exact(&mut crc32);
	let _ = file.read_exact(&mut reserved);
	let _ = file.read_exact(&mut current_lba);
	let _ = file.read_exact(&mut backup_lba);
	let _ = file.read_exact(&mut first_usable);
	let _ = file.read_exact(&mut last_usable);
	let _ = file.read_exact(&mut disk_guid);
	let _ = file.read_exact(&mut start_lba);
	let _ = file.read_exact(&mut num_parts);
	let _ = file.read_exact(&mut part_size);
	let _ = file.read_exact(&mut crc32_parts);

	return Ok(Header{
		signature: signature, 
		revision: revision, 
		header_size_le: header_size_le, 
		crc32: crc32, 
		reserved: reserved,
		current_lba: current_lba,
		backup_lba: backup_lba,
		first_usable: first_usable,
		last_usable: last_usable,
		disk_guid: parse_uuid(&disk_guid)?,
		start_lba: start_lba,
		num_parts: num_parts,
		part_size: part_size,
		crc32_parts: crc32_parts
	});
}
*/