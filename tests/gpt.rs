use gpt::{disk, GptConfig, GptError};

use std::io::{Cursor, Read, Seek, Write};
use tempfile::NamedTempFile;

#[test]
fn test_gptconfig_empty() {
    let mut tempdisk = NamedTempFile::new().expect("failed to create tempfile disk");
    tempdisk.write(&[0; 1024 * 64]).unwrap();
    let cfg = {
        let c1 = GptConfig::new();
        let c2 = GptConfig::default();
        assert_eq!(c1, c2);
        c1
    };

    let lb_size = disk::LogicalBlockSize::Lb4096;
    let disk = cfg
        .logical_block_size(lb_size)
        .create(tempdisk.path())
        .unwrap();
    assert_eq!(*disk.logical_block_size(), lb_size);
    assert!(disk.primary_header().is_ok());
    assert!(disk.backup_header().is_ok());
    assert!(disk.partitions().is_empty());
}

#[test]
fn test_create_simple_on_device() {
    const TOTAL_BYTES: usize = 1024 * 66;
    let mut mem_device = Box::new(std::io::Cursor::new(vec![0_u8; TOTAL_BYTES]));

    // Create a protective MBR at LBA0
    let mbr = gpt::mbr::ProtectiveMBR::with_lb_size(
        u32::try_from((TOTAL_BYTES / 512) - 1).unwrap_or(0xFF_FF_FF_FF),
    );
    mbr.overwrite_lba0(&mut mem_device).unwrap();

    let mut gdisk = GptConfig::default()
        .writable(true)
        .logical_block_size(disk::LogicalBlockSize::Lb512)
        .create_from_device(mem_device, None)
        .unwrap();

    gdisk
        .add_partition("test1", 1024 * 12, gpt::partition_types::BASIC, 0, None)
        .unwrap();
    gdisk
        .add_partition("test2", 1024 * 18, gpt::partition_types::LINUX_FS, 0, None)
        .unwrap();
    let id = gdisk.find_next_partition_id().unwrap();
    gdisk
        .add_partition_at("test3", id, 94, 2, gpt::partition_types::BASIC, 0)
        .unwrap();

    let mut mem_device = gdisk.write().unwrap();
    mem_device.seek(std::io::SeekFrom::Start(0)).unwrap();
    let mut final_bytes = vec![0_u8; TOTAL_BYTES];
    mem_device.read_exact(&mut final_bytes).unwrap();
}

fn t_read_bytes<D: gpt::DiskDevice>(device: &mut D, offset: u64, bytes: usize) -> Vec<u8> {
    let mut buf = vec![0_u8; bytes];
    device.seek(std::io::SeekFrom::Start(offset)).unwrap();
    device.read_exact(&mut buf).unwrap();
    buf
}

#[test]
fn test_only_valid_headers() {
    // write a valid disk
    let mut valid_disk = GptConfig::new()
        .writable(true)
        .create_from_device(Cursor::new(vec![0; 1024 * 70]), None)
        .unwrap();

    valid_disk
        .add_partition("test1", 1024 * 12, gpt::partition_types::BASIC, 0, None)
        .unwrap();
    valid_disk
        .add_partition("test2", 1024 * 18, gpt::partition_types::LINUX_FS, 0, None)
        .unwrap();
    let id = valid_disk.find_next_partition_id().unwrap();
    valid_disk
        .add_partition_at("test3", id, 94, 2, gpt::partition_types::BASIC, 0)
        .unwrap();
    // now write to memory
    let valid_disk = valid_disk.write().unwrap();
    let mut corrupt_disk = valid_disk.clone();
    corrupt_disk.get_mut()[..1024 * 32]
        .iter_mut()
        .for_each(|v| *v = 0);
    // override the first bytes so we need to read the backup header

    let first_try = GptConfig::new()
        .only_valid_headers(true)
        .open_from_device(&mut corrupt_disk);
    assert!(first_try.is_err());

    // lets try to write without changing any header
    let mut second_try = GptConfig::new()
        .writable(true)
        .open_from_device(corrupt_disk.clone())
        .unwrap();

    second_try.write_inplace().unwrap();
    assert_eq!(&valid_disk, second_try.device_ref());

    assert_eq!(second_try.partitions()[&1].name, "test1");
}

#[test]
fn test_readonly_backup() {
    // write a valid disk
    let mut valid_disk = GptConfig::new()
        .writable(true)
        .create_from_device(Cursor::new(vec![0; 1024 * 70]), None)
        .unwrap();

    valid_disk
        .add_partition("test1", 1024 * 12, gpt::partition_types::BASIC, 0, None)
        .unwrap();
    valid_disk
        .add_partition("test2", 1024 * 18, gpt::partition_types::LINUX_FS, 0, None)
        .unwrap();
    let id = valid_disk.find_next_partition_id().unwrap();
    valid_disk
        .add_partition_at("test4", id, 94, 2, gpt::partition_types::BASIC, 0)
        .unwrap();
    // now write to memory
    valid_disk.write_inplace().unwrap();
    let valid_disk_bytes = valid_disk.device_ref();
    let test_disk_bytes = valid_disk_bytes.clone();

    let mut test_disk = GptConfig::new()
        .writable(true)
        .readonly_backup(true)
        .open_from_device(test_disk_bytes)
        .unwrap();

    // change something
    test_disk
        .add_partition("test3", 1024 * 4, gpt::partition_types::LINUX_FS, 0, None)
        .unwrap();

    test_disk.write_inplace().unwrap();
    let test_disk_bytes = test_disk.device_ref();

    let primary_end = 512 * (1 + 1 + 32);
    assert_ne!(
        test_disk_bytes.get_ref()[..primary_end],
        valid_disk_bytes.get_ref()[..primary_end]
    );
    let backup_start = test_disk_bytes.get_ref().len() - 512 * (1 + 32);
    assert_eq!(
        test_disk_bytes.get_ref()[backup_start..],
        valid_disk_bytes.get_ref()[backup_start..]
    );
}

#[test]
fn test_change_partition_count() {
    let size = 67 + 128;

    // write a valid disk
    let mut valid_disk = GptConfig::new()
        .writable(true)
        .create_from_device(Cursor::new(vec![0; 512 * size]), None)
        .unwrap();

    // let's create 128 partitions
    for i in 0..128 {
        valid_disk
            .add_partition(
                &format!("test{i}"),
                512,
                gpt::partition_types::BASIC,
                0,
                None,
            )
            .unwrap();
    }

    let failed = valid_disk.add_partition(
        &format!("test129"),
        512,
        gpt::partition_types::BASIC,
        0,
        None,
    );
    assert!(matches!(failed, Err(GptError::PartitionCountWouldChange)));

    // now write to memory
    valid_disk.write_inplace().unwrap();

    // test when we are allowed to change
    let mut big_disk = GptConfig::new()
        .writable(true)
        .change_partition_count(true)
        .create_from_device(Cursor::new(vec![0; 512 * size]), None)
        .unwrap();

    // let's create 128 partitions
    for i in 0..129 {
        big_disk
            .add_partition(
                &format!("test{i}"),
                512,
                gpt::partition_types::BASIC,
                0,
                None,
            )
            .unwrap();
    }

    let data = big_disk.write().unwrap();

    let mut n_disk = GptConfig::new()
        .writable(true)
        .open_from_device(data.clone())
        .unwrap();
    n_disk.write_inplace().unwrap();

    assert_eq!(&data, n_disk.device_ref());

    // this would reduce the num parts let's make sure it does not cause a part Count WouldChange
    n_disk.remove_partition(129);
    n_disk.remove_partition(128);
    n_disk
        .add_partition("test128", 512, gpt::partition_types::BASIC, 0, None)
        .unwrap();

    n_disk.write_inplace().unwrap();

    assert_eq!(n_disk.header().num_parts, 129);
    assert_eq!(n_disk.partitions().len(), 128);
    assert_ne!(valid_disk.header().num_parts, n_disk.header().num_parts);
}

fn test_helper_gptdisk_write_efi_unused_partition_entries(lb_size: disk::LogicalBlockSize) {
    // Test that we write zeros to unused areas of the partition array, so that
    // if we're creating a partition table from scratch (not loading an existing
    // table and modifying it) it will create a partition array that is UEFI
    // compliant (has 128 entries) and unused entries are properly initialized
    // with zeros.

    let lb_bytes: u64 = lb_size.into();
    let lb_bytes_usize = lb_bytes as usize;
    // protective MBR + GPT header + GPT partition array
    let header_lbs = 1 + 1 + ((128 * 128) / lb_bytes);
    assert_eq!((128 * 128) % lb_bytes, 0);
    let data_lbs = 10;
    // GPT partition array + GPT header
    let footer_lbs = ((128 * 128) / lb_bytes) + 1;
    let total_lbs = header_lbs + data_lbs + footer_lbs;
    let total_bytes = (total_lbs * lb_bytes) as usize;

    // Initialize the buffer with all '255' values so we can tell what's been overwritten vs preserved.
    let mem_device = Box::new(std::io::Cursor::new(vec![255u8; total_bytes]));

    // Setup a new partition table and add a couple entries to it.
    let mut gdisk = GptConfig::default()
        .writable(true)
        .logical_block_size(lb_size)
        .create_from_device(mem_device, None)
        .unwrap();

    let part1_bytes = 3 * lb_bytes;
    gdisk
        .add_partition("test1", part1_bytes, gpt::partition_types::BASIC, 0, None)
        .unwrap();
    gdisk
        .add_partition(
            "test2",
            (data_lbs * lb_bytes) - part1_bytes,
            gpt::partition_types::LINUX_FS,
            0,
            None,
        )
        .unwrap();

    // Write out the table and get back the memory buffer so we can validate its contents.
    let mut mem_device = gdisk.write().unwrap();
    // Should NOT have overwritten the MBR (we have to generate a protective MBR explicitly using mbr module)
    assert_eq!(
        t_read_bytes(&mut mem_device, 0, lb_bytes_usize),
        vec![255u8; lb_bytes_usize]
    );
    // Should have overwritten the header
    assert_ne!(t_read_bytes(&mut mem_device, lb_bytes, 92), vec![255u8; 92]);
    // According to the spec, the rest of the sector containing the header should be zeros.
    assert_eq!(
        t_read_bytes(&mut mem_device, lb_bytes + 92, lb_bytes_usize - 92),
        vec![0_u8; lb_bytes_usize - 92]
    );
    // The first two partition entries should have been overwritten with non-zero data.
    let first_two = t_read_bytes(&mut mem_device, 2 * lb_bytes, 128 * 2);
    assert_ne!(first_two, vec![255u8; 128 * 2]);
    assert_ne!(first_two, vec![0_u8; 128 * 2]);
    // The remaining entries should have been overwritten with all zeros.
    assert_eq!(
        t_read_bytes(&mut mem_device, (2 * lb_bytes) + (128 * 2), 126 * 128),
        vec![0_u8; 126 * 128]
    );

    // The data area should be completely undisturbed...
    let data_bytes = (data_lbs as usize) * lb_bytes_usize;
    assert_eq!(
        t_read_bytes(&mut mem_device, header_lbs * lb_bytes, data_bytes),
        vec![255u8; data_bytes]
    );

    // The first two partition entries in the volume footer should have been overwritten with non-zero data.
    // The remaining entries should have been overwritten with all zeros.
    let first_two = t_read_bytes(&mut mem_device, (header_lbs + data_lbs) * lb_bytes, 128 * 2);
    assert_ne!(first_two, vec![255u8; 128 * 2]);
    assert_ne!(first_two, vec![0_u8; 128 * 2]);
    // The remaining entries should have been overwritten with all zeros.
    assert_eq!(
        t_read_bytes(&mut mem_device, (2 * lb_bytes) + (128 * 2), 126 * 128),
        vec![0_u8; 126 * 128]
    );

    // Should have overwritten the backup header
    assert_ne!(
        t_read_bytes(&mut mem_device, total_bytes as u64 - lb_bytes, 92),
        vec![255u8; 92]
    );
    // Remainder of the sector with the backup header should be all zeros
    assert_eq!(
        t_read_bytes(
            &mut mem_device,
            total_bytes as u64 - lb_bytes + 92,
            lb_bytes_usize - 92
        ),
        vec![0_u8; lb_bytes_usize - 92]
    );
}

#[test]
fn test_gptdisk_write_efi_unused_partition_entries_512() {
    test_helper_gptdisk_write_efi_unused_partition_entries(disk::LogicalBlockSize::Lb512);
}

#[test]
fn test_gptdisk_write_efi_unused_partition_entries_4096() {
    test_helper_gptdisk_write_efi_unused_partition_entries(disk::LogicalBlockSize::Lb4096);
}
