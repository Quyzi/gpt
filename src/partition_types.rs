extern crate uuid;

use std::collections::HashMap;

lazy_static! {
    pub static ref PART_HASHMAP: HashMap<String, (&'static str, &'static str)> = {
        let mut m = HashMap::new();
        m.insert(
            "00000000-0000-0000-0000-000000000000".into(),
            ("None", "Unused"),
        );
        m.insert(
            "024DEE41-33E7-11D3-9D69-0008C781F39F".into(),
            ("None", "MBR Partition Scheme"),
        );
        m.insert(
            "C12A7328-F81F-11D2-BA4B-00A0C93EC93B".into(),
            ("None", "EFI System Partition"),
        );
        m.insert(
            "21686148-6449-6E6F-744E-656564454649".into(),
            ("None", "BIOS Boot Partition"),
        );
        m.insert(
            "D3BFE2DE-3DAF-11DF-BA40-E3A556D89593".into(),
            ("None", "Intel Fast Flash (iFFS) Partition"),
        );
        m.insert(
            "F4019732-066E-4E12-8273-346C5641494F".into(),
            ("None", "Sony Boot Partition"),
        );
        m.insert(
            "BFBFAFE7-A34F-448A-9A5B-6213EB736C22".into(),
            ("None", "Lenovo Boot Partition"),
        );
        m.insert(
            "E3C9E316-0B5C-4DB8-817D-F92DF00215AE".into(),
            ("Windows", "Microsoft Reserved Partition"),
        );
        m.insert(
            "EBD0A0A2-B9E5-4433-87C0-68B6B72699C7".into(),
            ("Windows", "Basic Data Partition"),
        );
        m.insert(
            "5808C8AA-7E8F-42E0-85D2-E1E90434CFB3".into(),
            ("Windows", "Logical Disk Manager Metadata Partition"),
        );
        m.insert(
            "AF9B60A0-1431-4F62-BC68-3311714A69AD".into(),
            ("Windows", "Logical Disk Manager Data Partition"),
        );
        m.insert(
            "DE94BBA4-06D1-4D40-A16A-BFD50179D6AC".into(),
            ("Windows", "Windows Recovery Environment"),
        );
        m.insert(
            "37AFFC90-EF7D-4E96-91C3-2D7AE055B174".into(),
            ("Windows", "IBM General Parallel File System Partition"),
        );
        m.insert(
            "E75CAF8F-F680-4CEE-AFA3-B001E56EFC2D".into(),
            ("Windows", "Storage Spaces Partition"),
        );
        m.insert(
            "75894C1E-3AEB-11D3-B7C1-7B03A0000000".into(),
            ("HP-UX", "Data Partition"),
        );
        m.insert(
            "E2A1E728-32E3-11D6-A682-7B03A0000000".into(),
            ("HP-UX", "Service Partition"),
        );
        m.insert(
            "0FC63DAF-8483-4772-8E79-3D69D8477DE4".into(),
            ("Linux", "Linux Filesystem Data"),
        );
        m.insert(
            "A19D880F-05FC-4D3B-A006-743F0F84911E".into(),
            ("Linux", "RAID Partition"),
        );
        m.insert(
            "44479540-F297-41B2-9AF7-D131D5F0458A".into(),
            ("Linux", "Root Partition (x86)"),
        );
        m.insert(
            "4F68BCE3-E8CD-4DB1-96E7-FBCAF984B709".into(),
            ("Linux", "Root Partition (x86-64)"),
        );
        m.insert(
            "69DAD710-2CE4-4E3C-B16C-21A1D49ABED3".into(),
            ("Linux", "Root Partition (32-bit ARM)"),
        );
        m.insert(
            "B921B045-1DF0-41C3-AF44-4C6F280D3FAE".into(),
            ("Linux", "Root Partition (64-bit ARM/AArch64)"),
        );
        m.insert(
            "0657FD6D-A4AB-43C4-84E5-0933C84B4F4F".into(),
            ("Linux", "Swap Partition"),
        );
        m.insert(
            "E6D6D379-F507-44C2-A23C-238F2A3DF928".into(),
            ("Linux", "Logical Volume Manager Partition"),
        );
        m.insert(
            "933AC7E1-2EB4-4F13-B844-0E14E2AEF915".into(),
            ("Linux", "/home Partition"),
        );
        m.insert(
            "3B8F8425-20E0-4F3B-907F-1A25A76F98E8".into(),
            ("Linux", "/srv (Server Data) Partition"),
        );
        m.insert(
            "7FFEC5C9-2D00-49B7-8941-3EA10A5586B7".into(),
            ("Linux", "Plain dm-crypt Partition"),
        );
        m.insert(
            "CA7D7CCB-63ED-4C53-861C-1742536059CC".into(),
            ("Linux", "LUKS Partition"),
        );
        m.insert(
            "8DA63339-0007-60C0-C436-083AC8230908".into(),
            ("Linux", "Reserved"),
        );
        m.insert(
            "83BD6B9D-7F41-11DC-BE0B-001560B84F0F".into(),
            ("FreeBSD", "Boot Partition"),
        );
        m.insert(
            "516E7CB4-6ECF-11D6-8FF8-00022D09712B".into(),
            ("FreeBSD", "Data Partition"),
        );
        m.insert(
            "516E7CB5-6ECF-11D6-8FF8-00022D09712B".into(),
            ("FreeBSD", "Swap Partition"),
        );
        m.insert(
            "516E7CB6-6ECF-11D6-8FF8-00022D09712B".into(),
            ("FreeBSD", "Unix File System (UFS) Partition"),
        );
        m.insert(
            "516E7CB8-6ECF-11D6-8FF8-00022D09712B".into(),
            ("FreeBSD", "Vinium Volume Manager Partition"),
        );
        m.insert(
            "516E7CBA-6ECF-11D6-8FF8-00022D09712B".into(),
            ("FreeBSD", "ZFS Partition"),
        );
        m.insert(
            "48465300-0000-11AA-AA11-00306543ECAC".into(),
            (
                "macOS Darwin",
                "Hierarchical File System Plus (HFS+) Partition",
            ),
        );
        m.insert(
            "55465300-0000-11AA-AA11-00306543ECAC".into(),
            ("macOS Darwin", "Apple UFS"),
        );
        m.insert(
            "6A898CC3-1DD2-11B2-99A6-080020736631".into(),
            ("macOS Darwin", "ZFS"),
        );
        m.insert(
            "52414944-0000-11AA-AA11-00306543ECAC".into(),
            ("macOS Darwin", "Apple RAID Partition"),
        );
        m.insert(
            "52414944-5F4F-11AA-AA11-00306543ECAC".into(),
            ("macOS Darwin", "APple RAID Partition, offline"),
        );
        m.insert(
            "426F6F74-0000-11AA-AA11-00306543ECAC".into(),
            ("macOS Darwin", "Apple Boot Partition (Recovery HD)"),
        );
        m.insert(
            "4C616265-6C00-11AA-AA11-00306543ECAC".into(),
            ("macOS Darwin", "Apple Label"),
        );
        m.insert(
            "5265636F-7665-11AA-AA11-00306543ECAC".into(),
            ("macOS Darwin", "Apple TV Recovery Partition"),
        );
        m.insert(
            "53746F72-6167-11AA-AA11-00306543ECAC".into(),
            ("macOS Darwin", "Apple Core Storage Partition"),
        );
        m.insert(
            "B6FA30DA-92D2-4A9A-96F1-871EC6486200".into(),
            ("macOS Darwin", "SoftRAID_Status"),
        );
        m.insert(
            "2E313465-19B9-463F-8126-8A7993773801".into(),
            ("macOS Darwin", "SoftRAID_Scratch"),
        );
        m.insert(
            "FA709C7E-65B1-4593-BFD5-E71D61DE9B02".into(),
            ("macOS Darwin", "SoftRAID_Volume"),
        );
        m.insert(
            "BBBA6DF5-F46F-4A89-8F59-8765B2727503".into(),
            ("macOS Darwin", "SOftRAID_Cache"),
        );
        m.insert(
            "6A82CB45-1DD2-11B2-99A6-080020736631".into(),
            ("Solaris Illumos", "Boot Partition"),
        );
        m.insert(
            "6A85CF4D-1DD2-11B2-99A6-080020736631".into(),
            ("Solaris Illumos", "Root Partition"),
        );
        m.insert(
            "6A87C46F-1DD2-11B2-99A6-080020736631".into(),
            ("Solaris Illumos", "Swap Partition"),
        );
        m.insert(
            "6A8B642B-1DD2-11B2-99A6-080020736631".into(),
            ("Solaris Illumos", "Backup Partition"),
        );
        m.insert(
            "6A898CC3-1DD2-11B2-99A6-080020736631".into(),
            ("Solaris Illumos", "/usr Partition"),
        );
        m.insert(
            "6A8EF2E9-1DD2-11B2-99A6-080020736631".into(),
            ("Solaris Illumos", "/var Partition"),
        );
        m.insert(
            "6A90BA39-1DD2-11B2-99A6-080020736631".into(),
            ("Solaris Illumos", "/home Partition"),
        );
        m.insert(
            "6A9283A5-1DD2-11B2-99A6-080020736631".into(),
            ("Solaris Illumos", "Alternate Sector"),
        );
        m.insert(
            "6A945A3B-1DD2-11B2-99A6-080020736631".into(),
            ("Solaris Illumos", "Reserved"),
        );
        m.insert(
            "6A9630D1-1DD2-11B2-99A6-080020736631".into(),
            ("Solaris Illumos", "Reserved"),
        );
        m.insert(
            "6A980767-1DD2-11B2-99A6-080020736631".into(),
            ("Solaris Illumos", "Reserved"),
        );
        m.insert(
            "6A96237F-1DD2-11B2-99A6-080020736631".into(),
            (" Solaris Illumos", "Reserved"),
        );
        m.insert(
            "6A8D2AC7-1DD2-11B2-99A6-080020736631".into(),
            ("Solaris Illumos", "Reserved"),
        );
        m.insert(
            "49F48D32-B10E-11DC-B99B-0019D1879648".into(),
            ("NetBSD", "Swap Partition"),
        );
        m.insert(
            "49F48D5A-B10E-11DC-B99B-0019D1879648".into(),
            ("NetBSD", "FFS Partition"),
        );
        m.insert(
            "49F48D82-B10E-11DC-B99B-0019D1879648".into(),
            ("NetBSD", "LFS Partition"),
        );
        m.insert(
            "49F48DAA-B10E-11DC-B99B-0019D1879648".into(),
            ("NetBSD", "RAID Partition"),
        );
        m.insert(
            "2DB519C4-B10F-11DC-B99B-0019D1879648".into(),
            ("NetBSD", "Concatenated Partition"),
        );
        m.insert(
            "2DB519EC-B10F-11DC-B99B-0019D1879648".into(),
            ("NetBSD", "Encrypted Partition"),
        );
        m.insert(
            "FE3A2A5D-4F32-41A7-B725-ACCC3285A309".into(),
            ("ChromeOS", "ChromeOS Kernel"),
        );
        m.insert(
            "3CB8E202-3B7E-47DD-8A3C-7FF2A13CFCEC".into(),
            ("ChromeOS", "ChromeOS rootfs"),
        );
        m.insert(
            "2E0A753D-9E48-43B0-8337-B15192CB1B5E".into(),
            ("ChromeOS", "ChromeOS Future Use"),
        );
        m.insert(
            "5DFBF5F4-2848-4BAC-AA5E-0D9A20B745A6".into(),
            ("ContainerLinux by CoreOS", "/usr partition (coreos-usr)"),
        );
        m.insert(
            "3884DD41-8582-4404-B9A8-E9B84F2DF50E".into(),
            (
                "ContainerLinux by CoreOS",
                "Resizable rootfs (coreos-resize)",
            ),
        );
        m.insert(
            "C95DC21A-DF0E-4340-8D7B-26CBFA9A03E0".into(),
            (
                "ContainerLinux by CoreOS",
                "OEM customizations (coreos-reserved)",
            ),
        );
        m.insert(
            "BE9067B9-EA49-4F15-B4F6-F36F8C9E1818".into(),
            (
                "ContainerLinux by CoreOS",
                "Root filesystem on RAID (coreos-root-raid)",
            ),
        );
        m.insert(
            "42465331-3BA3-10F1-802A-4861696B7521".into(),
            ("Haiku", "Haiku BFS"),
        );
        m.insert(
            "85D5E45E-237C-11E1-B4B3-E89A8F7FC3A7".into(),
            ("MidnightBSD", "Boot Partition"),
        );
        m.insert(
            "85D5E45A-237C-11E1-B4B3-E89A8F7FC3A7".into(),
            ("MidnightBSD", "Data Partition"),
        );
        m.insert(
            "85D5E45B-237C-11E1-B4B3-E89A8F7FC3A7".into(),
            ("MidnightBSD", "Swap Partition"),
        );
        m.insert(
            "0394EF8B-237E-11E1-B4B3-E89A8F7FC3A7".into(),
            ("MidnightBSD", "Unix File System (UFS) Partition"),
        );
        m.insert(
            "85D5E45C-237C-11E1-B4B3-E89A8F7FC3A7".into(),
            ("MidnightBSD", "Vinium Volume Manager Partition"),
        );
        m.insert(
            "85D5E45D-237C-11E1-B4B3-E89A8F7FC3A7".into(),
            ("MidnightBSD", "ZFS Partition"),
        );
        m.insert(
            "45B0969E-9B03-4F30-B4C6-B4B80CEFF106".into(),
            ("Ceph", "Ceph Journal"),
        );
        m.insert(
            "45B0969E-9B03-4F30-B4C6-5EC00CEFF106".into(),
            ("Ceph", "Ceph dm-crypt Encryted Journal"),
        );
        m.insert(
            "4FBD7E29-9D25-41B8-AFD0-062C0CEFF05D".into(),
            ("Ceph", "Ceph OSD"),
        );
        m.insert(
            "4FBD7E29-9D25-41B8-AFD0-5EC00CEFF05D".into(),
            ("Ceph", "Ceph dm-crypt OSD"),
        );
        m.insert(
            "89C57F98-2FE5-4DC0-89C1-F3AD0CEFF2BE".into(),
            ("Ceph", "Ceph Disk In Creation"),
        );
        m.insert(
            "89C57F98-2FE5-4DC0-89C1-5EC00CEFF2BE".into(),
            ("Ceph", "Ceph dm-crypt Disk In Creation"),
        );
        m.insert(
            "824CC7A0-36A8-11E3-890A-952519AD3F61".into(),
            ("OpenBSD", "Data Partition"),
        );
        m.insert(
            "CEF5A9AD-73BC-4601-89F3-CDEEEEE321A1".into(),
            ("QNX", "Power-safe (QNX6) File System"),
        );
        m.insert(
            "C91818F9-8025-47AF-89D2-F030D7000C2C".into(),
            ("Plan 9", "Plan 9 Partition"),
        );
        m.insert(
            "9D275380-40AD-11DB-BF97-000C2911D1B8".into(),
            ("VMware ESX", "vmkcore (coredump partition)"),
        );
        m.insert(
            "AA31E02A-400F-11DB-9590-000C2911D1B8".into(),
            ("VMware ESX", "VMFS Filesystem Partition"),
        );
        m.insert(
            "9198EFFC-31C0-11DB-8F78-000C2911D1B8".into(),
            ("VMware ESX", "VMware Reserved"),
        );
        m.insert(
            "2568845D-2332-4675-BC39-8FA5A4748D15".into(),
            ("Android-IA", "Bootloader"),
        );
        m.insert(
            "114EAFFE-1552-4022-B26E-9B053604CF84".into(),
            ("Android-IA", "Bootloader2"),
        );
        m.insert(
            "49A4D17F-93A3-45C1-A0DE-F50B2EBE2599".into(),
            ("Android-IA", "Boot"),
        );
        m.insert(
            "4177C722-9E92-4AAB-8644-43502BFD5506".into(),
            ("Android-IA", "Recovery"),
        );
        m.insert(
            "EF32A33B-A409-486C-9141-9FFB711F6266".into(),
            ("Android-IA", "Misc"),
        );
        m.insert(
            "20AC26BE-20B7-11E3-84C5-6CFDB94711E9".into(),
            ("Android-IA", "Metadata"),
        );
        m.insert(
            "38F428E6-D326-425D-9140-6E0EA133647C".into(),
            ("Android-IA", "System"),
        );
        m.insert(
            "A893EF21-E428-470A-9E55-0668FD91A2D9".into(),
            ("Android-IA", "Cache"),
        );
        m.insert(
            "DC76DDA9-5AC1-491C-AF42-A82591580C0D".into(),
            ("Android-IA", "Data"),
        );
        m.insert(
            "EBC597D0-2053-4B15-8B64-E0AAC75F4DB1".into(),
            ("Android-IA", "Persistent"),
        );
        m.insert(
            "8F68CC74-C5E5-48DA-BE91-A0C8C15E9C80".into(),
            ("Android-IA", "Factory"),
        );
        m.insert(
            "767941D0-2085-11E3-AD3B-6CFDB94711E9".into(),
            ("Android-IA", "Fastboot/Tertiary"),
        );
        m.insert(
            "AC6D7924-EB71-4DF8-B48D-E267B27148FF".into(),
            ("Android-IA", "OEM"),
        );
        m.insert(
            "7412F7D5-A156-4B13-81DC-867174929325".into(),
            ("ONIE", "Boot"),
        );
        m.insert(
            "D4E6E2CD-4469-46F3-B5CB-1BFF57AFC149".into(),
            ("ONIE", "Config"),
        );
        m.insert(
            "9E1A2D38-C612-4316-AA26-8B49521E5A8B".into(),
            ("PowerPC", "PReP Boot"),
        );
        m.insert(
            "BC13C2FF-59E6-4262-A352-B275FD6F7172".into(),
            ("Freedesktop", "Shared Boot Loader Configuration"),
        );
        m.insert(
            "734E5AFE-F61A-11E6-BC64-92361F002671".into(),
            ("Atari TOS", "Basic Data Partition (GEM, BGM, F32)"),
        );
        m
    };
}
