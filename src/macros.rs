
#[macro_use]
pub mod macros {
    /// Macro to read an exact buffer
    #[macro_export]
    macro_rules! read_exact_buff {
        ($bufid:ident, $rdr:expr, $buflen:expr) => {
            {
                let mut $bufid = [0u8; $buflen];
                let _ = $rdr.read_exact(&mut $bufid)?;
                $bufid
            }
        }
    }

    macro_rules! partition_types {
    (
        $(
            $(#[$docs:meta])*
            ($upcase:ident, $guid:expr, $os:expr)$(,)*
        )+
    ) => {
        $(
            $(#[$docs])*
            pub const $upcase: Type = Type {
                guid: $guid,
                os: $os,
            };
        )+

        impl FromStr for Type {
            type Err = String;
            fn from_str(s: &str) -> Result<Self, Self::Err> {
                match s {
                    $(
                        $guid => Ok(Type { guid: $guid, os: $os }),
                    )+
                    _ => Err("Invalid or unknown Partition Type GUID.".to_string()),
                }
            }
        }
    }
}
}