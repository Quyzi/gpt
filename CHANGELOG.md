### v4.1.0 (2025-03-16)

#### Changes
- After writing to disk sync_all is now called to ensure the data is written to disk #108

#### Fixes
- Partitions would overlap if `size <= lb_size` #109

### v4.0.0 (2024-09-13)

#### Behaviour changes

- Opening a `GptDisk` now succeeds even if one header is invalid
  (use the `only_valid_headers` config flag to get the old behaviour back)

#### Changes

- `Type::from_str` now is case insensitive, Thanks @IronBatman2715
- add `GptDisk::calculate_alignment` allowing to calculate the sector alignment, Thanks @gaochuntie
- add `GptDisk::add_partition_at`, Thanks @gaochuntie
- Bump MSRV to 1.65
- implement Clone for GptDisk
- relax trait bounds on some functions of GptDisk
- remove `Partition::size` and replace it with `sectors_len` which returns the correct number of sectors, Thanks @sjoerdsimons
- `GptDisk::{primary_header, backup_header, try_header}` now return an error instead of just `Option`
- Remove initialized state & configuration
- add the option to allow the partition count to be changed
- add the option to only open a disk if both headers are valid
- add the option to keep the backup partition readonly
- split `GptDisk::remove_partition` into two functions `remove_partition` and `remove_partition_by_guid`
- add `GptDisk::header` function which allows to get the current header (either primary or backup)
- add `GptDisk::take_partitions`
- remove `GptDisk::update_partitions_safe` and replace it with a config option `readonly_backup`
- remove `GptDisk::update_partitions_embedded` and replace it with the config option `change_partition_count`
- add `GptDisk::device_ref`
- add `GptDisk::device_mut`
- crc32 are now stored in the header after it has been written instead of always being zeros
- Add DragonFlyBSD as partition and OS type, Thanks @phcoder
- `GptDisk` now accepts a generic `DiskDevice`
- add ChromeOS RWFW partition type, Thanks @phcoder
- improve error reporting, returning `HeaderError` or the new `GptError`
- add `HeaderBuilder` to simplify creating a header (replaces Header::compute_new)
- add `GptDisk::take_device`
- Support custom partition GUIDs
- logging is now optional use the `log` or `tracing` feature to use the appropriate logging crate
