//! GPT-header object and helper functions.

mod builder;

pub use builder::HeaderBuilder;

use crc::Crc;
use log::*;
use std::fmt;
use std::fs::{File, OpenOptions};
use std::io::{Error, ErrorKind, Read, Result, Seek, SeekFrom, Write};
use std::path::Path;

use crate::disk;

use simple_bytes::{BytesArray, BytesRead, BytesSeek, BytesWrite};

#[non_exhaustive]
#[derive(Debug, Clone)]
/// Errors returned when interacting with a header.
pub enum HeaderError {
    // Builder errors
    /// Get's returned when you call build on a HeaderBuilder and the backup lba field
    /// was never set
    MissingBackupLba,
    /// Get's returned when you call build on a HeaderBuilder and there isn't enough space
    /// between first_lba and backup_lba
    BackupLbaToEarly,
}

impl std::error::Error for HeaderError {}

impl fmt::Display for HeaderError {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        use HeaderError::*;
        let desc = match self {
            MissingBackupLba => "HeaderBuilder expects the field backup_lba to be set",
            BackupLbaToEarly => {
                "HeaderBuilder: there isn't enough space between first_lba and backup_lba"
            }
        };
        write!(fmt, "{}", desc)
    }
}

/// Header describing a GPT disk.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Header {
    /// GPT header magic signature, hardcoded to "EFI PART".
    pub signature: String, // Offset  0. "EFI PART", 45h 46h 49h 20h 50h 41h 52h 54h
    /// major, minor
    pub revision: (u16, u16), // Offset  8
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
    /// Last usable LBA (secondary partition table first LBA - 1)
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
    /// Write the primary header.
    ///
    /// With a CRC32 set to zero this will set the crc32 after
    /// writing the header out.
    pub fn write_primary<D: Read + Write + Seek>(
        &self,
        file: &mut D,
        lb_size: disk::LogicalBlockSize,
    ) -> Result<usize> {
        // This is the primary header. It must start before the backup one.
        if self.current_lba >= self.backup_lba {
            debug!(
                "current lba: {} backup_lba: {}",
                self.current_lba, self.backup_lba
            );
            return Err(Error::new(
                ErrorKind::Other,
                "primary header does not start before backup one",
            ));
        }
        self.file_write_header(file, self.current_lba, lb_size)
    }

    /// Write the backup header.
    ///
    /// With a CRC32 set to zero this will set the crc32 after
    /// writing the header out.
    pub fn write_backup<D: Read + Write + Seek>(
        &self,
        file: &mut D,
        lb_size: disk::LogicalBlockSize,
    ) -> Result<usize> {
        // This is the backup header. It must start after the primary one.
        if self.current_lba <= self.backup_lba {
            debug!(
                "current lba: {} backup_lba: {}",
                self.current_lba, self.backup_lba
            );
            return Err(Error::new(
                ErrorKind::Other,
                "backup header does not start after primary one",
            ));
        }
        self.file_write_header(file, self.current_lba, lb_size)
    }

    /// Write an header to an arbitrary LBA.
    ///
    /// With a CRC32 set to zero this will set the crc32 after
    /// writing the header out.
    fn file_write_header<D: Read + Write + Seek>(
        &self,
        file: &mut D,
        lba: u64,
        lb_size: disk::LogicalBlockSize,
    ) -> Result<usize> {
        // Build up byte array in memory
        let parts_checksum = partentry_checksum(file, self, lb_size)?;
        trace!("computed partitions CRC32: {:#x}", parts_checksum);
        let bytes = self.as_bytes(None, Some(parts_checksum))?;
        trace!("bytes before checksum: {:?}", bytes);

        // Calculate the CRC32 from the byte array
        let checksum = calculate_crc32(&bytes);
        trace!("computed header CRC32: {:#x}", checksum);

        // Write it to disk in 1 shot
        let start = lba
            .checked_mul(lb_size.into())
            .ok_or_else(|| Error::new(ErrorKind::Other, "header overflow - offset"))?;
        trace!("Seeking to {}", start);
        let _ = file.seek(SeekFrom::Start(start))?;
        let header_bytes = self.as_bytes(Some(checksum), Some(parts_checksum))?;
        // Per the spec, the rest of the logical block must be zeros...
        let mut bytes = Vec::with_capacity(lb_size.as_usize());
        bytes.extend_from_slice(&header_bytes);
        bytes.resize(lb_size.as_usize(), 0);
        let len = file.write(&bytes)?;
        trace!("Wrote {} bytes", len);

        Ok(len)
    }

    fn as_bytes(
        &self,
        header_checksum: Option<u32>,
        partitions_checksum: Option<u32>,
    ) -> Result<[u8; 92]> {
        let mut bytes = BytesArray::from([0u8; 92]);
        let disk_guid_fields = self.disk_guid.as_fields();

        BytesWrite::write(&mut bytes, self.signature.as_bytes());
        bytes.write_le_u16(self.revision.1);
        bytes.write_le_u16(self.revision.0);
        bytes.write_le_u32(self.header_size_le);
        bytes.write_le_u32(header_checksum.unwrap_or_default());
        bytes.write_le_u32(0);
        bytes.write_le_u64(self.current_lba);
        bytes.write_le_u64(self.backup_lba);
        bytes.write_le_u64(self.first_usable);
        bytes.write_le_u64(self.last_usable);
        bytes.write_le_u32(disk_guid_fields.0);
        bytes.write_le_u16(disk_guid_fields.1);
        bytes.write_le_u16(disk_guid_fields.2);
        BytesWrite::write(&mut bytes, disk_guid_fields.3);
        bytes.write_le_u64(self.part_start);
        bytes.write_le_u32(self.num_parts);
        bytes.write_le_u32(self.part_size);
        bytes.write_le_u32(partitions_checksum.unwrap_or_default());

        Ok(bytes.into_array())
    }
}

/// Parses a uuid with first 3 portions in little endian.
pub fn parse_uuid<R: BytesRead>(rdr: &mut R) -> Result<uuid::Uuid> {
    if rdr.remaining().len() < 16 {
        return Err(Error::new(ErrorKind::UnexpectedEof, "uuid needs 16bytes"));
    }

    let d1 = rdr.read_le_u32();
    let d2 = rdr.read_le_u16();
    let d3 = rdr.read_le_u16();
    let d4 = rdr.read(8).try_into().unwrap();

    let uuid = uuid::Uuid::from_fields(d1, d2, d3, &d4);
    Ok(uuid)
}

impl fmt::Display for Header {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Disk:\t\t{}\nCRC32:\t\t{}\nTable CRC:\t{}",
            self.disk_guid, self.crc32, self.crc32_parts
        )
    }
}

/// Read a GPT header from a given path.
///
/// ## Example
///
/// ```rust,no_run
/// use gpt::header::read_header;
///
/// let lb_size = gpt::disk::DEFAULT_SECTOR_SIZE;
/// let diskpath = std::path::Path::new("/dev/sdz");
///
/// let h = read_header(diskpath, lb_size).unwrap();
/// ```
pub fn read_header(path: impl AsRef<Path>, sector_size: disk::LogicalBlockSize) -> Result<Header> {
    let mut file = File::open(path)?;
    read_primary_header(&mut file, sector_size)
}

/// Read a GPT header from any device capable of reading and seeking.
pub fn read_header_from_arbitrary_device<D: Read + Seek>(
    device: &mut D,
    sector_size: disk::LogicalBlockSize,
) -> Result<Header> {
    read_primary_header(device, sector_size)
}

pub(crate) fn read_primary_header<D: Read + Seek>(
    file: &mut D,
    sector_size: disk::LogicalBlockSize,
) -> Result<Header> {
    let cur = file.seek(SeekFrom::Current(0)).unwrap_or(0);
    let offset: u64 = sector_size.into();
    let res = file_read_header(file, offset);
    let _ = file.seek(SeekFrom::Start(cur));
    res
}

pub(crate) fn read_backup_header<D: Read + Seek>(
    file: &mut D,
    sector_size: disk::LogicalBlockSize,
) -> Result<Header> {
    let cur = file.seek(SeekFrom::Current(0)).unwrap_or(0);
    let h2sect = find_backup_lba(file, sector_size)?;
    let offset = h2sect
        .checked_mul(sector_size.into())
        .ok_or_else(|| Error::new(ErrorKind::Other, "backup header overflow - offset"))?;
    let res = file_read_header(file, offset);
    let _ = file.seek(SeekFrom::Start(cur));
    res
}

pub(crate) fn file_read_header<D: Read + Seek>(file: &mut D, offset: u64) -> Result<Header> {
    let _ = file.seek(SeekFrom::Start(offset));

    let mut bytes = BytesArray::from([0u8; 92]);
    file.read_exact(bytes.as_mut())?;

    let sigstr = String::from_utf8_lossy(BytesRead::read(&mut bytes, 8)).into_owned();

    if sigstr != "EFI PART" {
        return Err(Error::new(ErrorKind::Other, "invalid GPT signature"));
    };

    let h = Header {
        signature: sigstr,
        revision: {
            let minor = bytes.read_le_u16();
            let major = bytes.read_le_u16();
            (major, minor)
        },
        header_size_le: bytes.read_le_u32(),
        crc32: bytes.read_le_u32(),
        reserved: bytes.read_le_u32(),
        current_lba: bytes.read_le_u64(),
        backup_lba: bytes.read_le_u64(),
        first_usable: bytes.read_le_u64(),
        last_usable: bytes.read_le_u64(),
        disk_guid: parse_uuid(&mut bytes)?,
        part_start: bytes.read_le_u64(),
        // Note: this will always return the total number of partition entries
        // in the array, not how many are actually used
        num_parts: bytes.read_le_u32(),
        part_size: bytes.read_le_u32(),
        crc32_parts: bytes.read_le_u32(),
    };
    trace!("header: {:?}", bytes.as_slice());
    trace!("header gpt: {}", h.disk_guid.as_hyphenated().to_string());

    // override crc32
    BytesSeek::seek(&mut bytes, 16);
    bytes.write_u32(0);

    let c = calculate_crc32(bytes.as_slice());
    trace!("header CRC32: {:#x} - computed CRC32: {:#x}", h.crc32, c);
    if c == h.crc32 {
        Ok(h)
    } else {
        Err(Error::new(ErrorKind::Other, "invalid CRC32 checksum"))
    }
}

pub(crate) fn find_backup_lba<D: Read + Seek>(
    f: &mut D,
    sector_size: disk::LogicalBlockSize,
) -> Result<u64> {
    trace!("querying file size to find backup header location");
    let lb_size: u64 = sector_size.into();
    let old_pos = f.seek(std::io::SeekFrom::Current(0))?;
    let len = f.seek(std::io::SeekFrom::End(0))?;
    f.seek(std::io::SeekFrom::Start(old_pos))?;
    // lba0: prot mbr, lba1: prim, .., lba-1: backup
    // at least three lba need to be present else it doesn't make sense
    // to check for the backup header
    if len < lb_size * 3 {
        return Err(Error::new(
            ErrorKind::Other,
            "disk image too small for backup header",
        ));
    }
    let bak_offset = len.saturating_sub(lb_size);
    let bak_lba = bak_offset / lb_size;
    trace!(
        "backup header: LBA={}, bytes offset={}",
        bak_lba,
        bak_offset
    );

    Ok(bak_lba)
}

const CRC_32: Crc<u32> = Crc::<u32>::new(&crc::CRC_32_ISO_HDLC);

fn calculate_crc32(b: &[u8]) -> u32 {
    let mut digest = CRC_32.digest();
    trace!("Writing buffer to digest calculator");
    digest.update(b);

    digest.finalize()
}

pub(crate) fn partentry_checksum<D: Read + Seek>(
    file: &mut D,
    hdr: &Header,
    lb_size: disk::LogicalBlockSize,
) -> Result<u32> {
    // Seek to start of partition table.
    trace!("Computing partition checksum");
    let start = hdr
        .part_start
        .checked_mul(lb_size.into())
        .ok_or_else(|| Error::new(ErrorKind::Other, "header overflow - partition table start"))?;
    trace!("Seek to {}", start);
    let _ = file.seek(SeekFrom::Start(start))?;

    // Read partition table.
    let pt_len = u64::from(hdr.num_parts)
        .checked_mul(hdr.part_size.into())
        .ok_or_else(|| Error::new(ErrorKind::Other, "partition table - size"))?;
    trace!("Reading {} bytes", pt_len);
    let mut buf = vec![0; pt_len as usize];
    file.read_exact(&mut buf)?;

    //trace!("Buffer before checksum: {:?}", buf);
    // Compute CRC32 over all table bits.
    Ok(calculate_crc32(&buf))
}

/// A helper function to create a new header and write it to disk.
/// If the uuid isn't given a random one will be generated.  Use
/// this in conjunction with Partition::write()
// TODO: Move this to Header::new() and Header::write to write it
// that will match the Partition::write() API
pub fn write_header(
    p: impl AsRef<Path>,
    uuid: Option<uuid::Uuid>,
    sector_size: disk::LogicalBlockSize,
) -> Result<uuid::Uuid> {
    debug!("opening {} for writing", p.as_ref().display());
    let mut file = OpenOptions::new().write(true).read(true).open(p)?;
    let bak = find_backup_lba(&mut file, sector_size)?;

    let mut header = HeaderBuilder::new();

    if let Some(uuid) = uuid {
        header.disk_guid(uuid);
    }

    let header = header
        .backup_lba(bak)
        .build(sector_size)
        .map_err(|e| Error::new(ErrorKind::Other, e))?;

    debug!("new header: {:#?}", header);
    header.write_primary(&mut file, sector_size)?;

    Ok(header.disk_guid)
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::disk::LogicalBlockSize;

    use std::fs;
    use std::io::Cursor;

    /// whats needs to be tested
    /// creating
    /// reading
    /// writing

    /// edgecases
    /// part_size different
    /// num_parts different
    /// same part start

    fn expected_headers() -> (Header, Header) {
        let expected_primary = Header {
            signature: "EFI PART".to_string(),
            revision: (1, 0),
            header_size_le: 92,
            crc32: 0x55f06699,
            reserved: 0,
            current_lba: 1,
            backup_lba: 71,
            first_usable: 34,
            last_usable: 38,
            disk_guid: "1B6A2BFA-E92B-184C-A8A7-ED0610D54821".parse().unwrap(),
            part_start: 2,
            num_parts: 128,
            part_size: 128,
            crc32_parts: 0x5fad601b,
        };

        let mut expected_backup = expected_primary.clone();
        expected_backup.crc32 = 0x7ddfa41b;
        expected_backup.current_lba = 71;
        expected_backup.backup_lba = 1;
        expected_backup.part_start = 39;

        (expected_primary, expected_backup)
    }

    #[test]
    fn read_gpt_disk() {
        let lb_size = LogicalBlockSize::Lb512;
        let diskpath = Path::new("tests/fixtures/gpt-disk.img");

        let (expected_primary, expected_backup) = expected_headers();

        let mut file = File::open(diskpath).unwrap();
        let primary = read_primary_header(&mut file, lb_size).unwrap();
        let backup = read_backup_header(&mut file, lb_size).unwrap();

        assert_eq!(primary, expected_primary);
        assert_eq!(backup, expected_backup);
    }

    #[test]
    fn create_gpt_disk() {
        let header_1 = HeaderBuilder::new()
            .disk_guid("1B6A2BFA-E92B-184C-A8A7-ED0610D54821".parse().unwrap())
            .backup_lba(71)
            .build(LogicalBlockSize::Lb512)
            .unwrap();

        let backup_header = HeaderBuilder::new()
            .disk_guid("1B6A2BFA-E92B-184C-A8A7-ED0610D54821".parse().unwrap())
            .backup_lba(71)
            .primary(false)
            .build(LogicalBlockSize::Lb512)
            .unwrap();

        let (mut expected_primary, mut expected_backup) = expected_headers();

        let header_2 = HeaderBuilder::from_header(&expected_primary)
            .build(LogicalBlockSize::Lb512)
            .unwrap();

        assert_eq!(header_1, header_2);

        expected_primary.crc32 = 0;
        expected_primary.crc32_parts = 0;
        expected_backup.crc32 = 0;
        expected_backup.crc32_parts = 0;

        assert_eq!(expected_primary, header_1);
        assert_eq!(expected_backup, backup_header);
    }

    #[test]
    fn write_gpt_disk() {
        let lb_size = LogicalBlockSize::Lb512;

        let primary = HeaderBuilder::new()
            .disk_guid("1B6A2BFA-E92B-184C-A8A7-ED0610D54821".parse().unwrap())
            .backup_lba(71)
            .build(lb_size)
            .unwrap();

        let backup = HeaderBuilder::new()
            .disk_guid("1B6A2BFA-E92B-184C-A8A7-ED0610D54821".parse().unwrap())
            .backup_lba(71)
            .primary(false)
            .build(lb_size)
            .unwrap();

        let diskpath = Path::new("tests/fixtures/gpt-disk.img");
        let mut expected_disk = Cursor::new(fs::read(diskpath).unwrap());
        let mut memory_disk = expected_disk.clone();

        let first_lba = 1;
        let backup_lba = find_backup_lba(&mut expected_disk, lb_size).unwrap();

        let primary_bytes = [0u8; 92];
        let backup_bytes = [0u8; 92];

        // clear out primary and backup
        memory_disk
            .seek(SeekFrom::Start(first_lba * lb_size.as_u64()))
            .unwrap();
        memory_disk.write_all(&primary_bytes).unwrap();
        memory_disk
            .seek(SeekFrom::Start(backup_lba * lb_size.as_u64()))
            .unwrap();
        memory_disk.write_all(&backup_bytes).unwrap();

        primary.write_primary(&mut memory_disk, lb_size).unwrap();
        backup.write_backup(&mut memory_disk, lb_size).unwrap();

        assert_eq!(memory_disk.into_inner(), expected_disk.into_inner());
    }
}
