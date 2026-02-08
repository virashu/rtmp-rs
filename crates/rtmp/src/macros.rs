macro_rules! primitive_enum {
    (
        #[repr($primitive_type:ident)]
        $(#[$meta:meta])*
        $vis:vis enum $name:ident {
            $(
                $(#[$vmeta:meta])*
                $vname:ident $(= $val:expr)?
            ),*
            $(,)?
        }
    ) => {
        #[repr($primitive_type)]
        $(#[$meta])*
        $vis enum $name {
            $(
                $(#[$vmeta])*
                $vname $(= $val)?
            ),*
        }

        impl ::std::convert::TryFrom<$primitive_type> for $name {
            type Error = ::anyhow::Error;

            fn try_from(value: $primitive_type) -> ::std::result::Result<Self, Self::Error> {
                match value {
                    $(
                        x if x == $name::$vname as $primitive_type => Ok($name::$vname),
                    )*
                    _ => Err(anyhow::anyhow!("Unknown value: {value:?}")),
                }
            }
        }

        impl ::std::convert::From<$name> for $primitive_type {
            fn from(value: $name) -> $primitive_type {
                value as $primitive_type
            }
        }
    }
}
pub(crate) use primitive_enum;
