#[macro_use]
pub mod pub_macros {

    /// Macro to create const for partition types.
    macro_rules! partition_types {
        (
            $(
                $(#[$docs:meta])*
                ($upcase:ident, $guid:expr, $os:expr)$(,)*
            )+
        ) => {
            const fn str_to_uuid_or_panic(s: &str) -> Uuid {
                let res_u = Uuid::try_parse(s);
                match res_u {
                    Ok(u) => return u,
                    Err(_) => {
                        // const_panic requires 1.57
                        #[allow(unconditional_panic)]
                        ["string was not an uuid"][1];
                        // never type
                        loop {}
                    },
                }
            }
            $(
                $(#[$docs])*
                pub const $upcase: Type = Type {
                    guid: str_to_uuid_or_panic($guid),
                    os: $os,
                };
            )+

            impl FromStr for Type {
                type Err = String;
                fn from_str(s: &str) -> Result<Self, Self::Err> {
                    match s {
                        $(
                            $guid |
                            stringify!($upcase) => Ok($upcase),
                        )+
                        _ => {
                            match ::uuid::Uuid::from_str(s) {
                                Ok(u) => Ok(Type {
                                    guid: u,
                                    os: OperatingSystem::None,
                                }),
                                Err(_) => Err("Invalid Partition Type GUID.".to_string()),
                            }
                        }
                    }
                }
            }
            impl From<Uuid> for Type {
                fn from(guid: Uuid) -> Self {
                    $(
                        if guid == $upcase.guid {
                            return $upcase;
                        }
                    )+
                    Type {
                        guid,
                        os: OperatingSystem::None,
                    }
                }
            }
        }
    }
}

pub(crate) trait ResultInsert<T> {
    fn insert_ok(&mut self, value: T) -> &mut T;
}

impl<T, E> ResultInsert<T> for Result<T, E> {
    fn insert_ok(&mut self, value: T) -> &mut T {
        *self = Ok(value);

        // SAFETY: the code above just filled the option
        unsafe { self.as_mut().unwrap_unchecked() }
    }
}
