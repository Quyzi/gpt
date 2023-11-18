### v4.0.0-rc.1 (2023-11-18)

#### Behaviour changes

* Opening a `GptDisk` now succeeds even if one header is invalid
  (use the `only_valid_headers` config flag to get the old behaviour back)

#### Changes

* Bump MSRV to 1.63

* Remove initialized state & configuration
* add the option to allow the partition count to be changed
* add the option to only open a disk if both headers are valid
* add the option to keep the backup partition readonly
* split `GptDisk::remove_partition` into two functions `remove_partition` and `remove_partition_by_guid`
* add `GptDisk::header` function which allows to get the current header (either primary or backup)
* add `GptDisk::take_partitions`
* remove `GptDisk::update_partitions_safe` and replace it with a config option `readonly_backup`
* remove `GptDisk::update_partitions_embedded` and replace it with the config option `change_partition_count`
* add `GptDisk::device_ref`
* add `GptDisk::device_mut`
* crc32 are now stored in the header after it has been written instead of always being zeros
* Add DragonFlyBSD as partition and OS type
* `GptDisk` now accepts a generic `DiskDevice`
* add ChromeOS RWFW partition type
* improve error reporting, returning `HeaderError` or the new `GptError`
* add `HeaderBuilder` to simplify creating a header (replaces Header::compute_new)
* add `GptDisk::take_device`
* Support custom partition GUIDs