use std::fs::File;
use std::io::{Read, Seek, Cursor, SeekFrom, Error, ErrorKind};

use header::{Header, parse_uuid};

extern crate uuid;
extern crate byteorder;
extern crate crc;
extern crate serde;
extern crate serde_json;

use self::byteorder::{LittleEndian, ReadBytesExt};
use self::uuid::Uuid;
use self::crc::crc32;

#[derive(Debug)]
pub struct Partition {
	part_type_guid: PartitionType,
	part_guid: uuid::Uuid,
	first_LBA: u32, // little endian
	last_LBA: u32,
	flags: u32,
	name: String // 36 UTF-16LE code units
}

#[derive(Debug, Serialize, Deserialize)]
struct PartitionType {
	os: String,
	guid: String,
	desc: String
}

fn read_part_name(rdr: &mut Cursor<&[u8]>) -> String {
	let mut namebytes: Vec<u16> = Vec::new();
	for _ in 0..36 {
		let b = rdr.read_u16::<LittleEndian>().unwrap();
		if b != 0 {
			namebytes.push(b);
		}
	}

	return String::from_utf16_lossy(&namebytes);
}

fn parse_parttype_guid(str: uuid::Uuid) -> Result<PartitionType, Error> {
	let uuid = str.hyphenated().to_string().to_uppercase();
	let mut file = File::open("types.json")?;
	let mut json: String = String::new();
	let _ = file.read_to_string(&mut json);
	let mut guids: Vec<PartitionType> = serde_json::from_str(&json)?;

	Ok(PartitionType {os: String::from("Dummy"), desc: String::from("Dummy"), guid: uuid})

}

pub fn read_partitions(path: &String, header: &Header) -> Result<Vec<Partition>, Error> {
	let mut file = File::open(path)?;
    let _ = file.seek(SeekFrom::Start(512 * header.part_start));

    let mut parts: Vec<Partition> = Vec::new();

    for _ in 0..header.num_parts {
    	let mut bytes: [u8; 56] = [0; 56];
    	let mut nameraw: [u8; 72] = [0; 72]; 

    	let _ = file.read_exact(&mut bytes);
    	let _ = file.read_exact(&mut nameraw);
    	let partname = read_part_name(&mut Cursor::new(&nameraw[..]));

    	let mut reader = Cursor::new(&bytes[..]);

    	let p: Partition = Partition {
    		part_type_guid: parse_parttype_guid(parse_uuid(&mut reader)?)?,
    		part_guid: parse_uuid(&mut reader)?,
    		first_LBA: reader.read_u32::<LittleEndian>()?,
    		last_LBA: reader.read_u32::<LittleEndian>()?,
    		flags: reader.read_u32::<LittleEndian>()?,
    		name: partname.to_string()
    	};

    	if p.part_guid.simple().to_string() != "00000000000000000000000000000000" {
    		parts.push(p);
    	}
    }

    Ok(parts)
}