
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
}