/// Defines a string enum that keeps values it does not know.
///
/// The API adds new enum values over time. A value this version of the crate
/// has never seen lands in `Unknown(String)` instead of failing the whole
/// response, and it serializes back to the same string.
macro_rules! string_enum {
    (
        $(#[$meta:meta])*
        pub enum $name:ident {
            $(
                $(#[$vmeta:meta])*
                $variant:ident = $value:literal,
            )+
        }
    ) => {
        $(#[$meta])*
        #[derive(Debug, Clone, PartialEq, Eq, Hash)]
        #[non_exhaustive]
        pub enum $name {
            $(
                $(#[$vmeta])*
                $variant,
            )+
            /// A value added to the API after this version of the crate.
            /// It holds the string exactly as the server sent it.
            Unknown(String),
        }

        impl $name {
            /// The value as the API writes it.
            pub fn as_str(&self) -> &str {
                match self {
                    $(Self::$variant => $value,)+
                    Self::Unknown(s) => s.as_str(),
                }
            }
        }

        impl ::std::fmt::Display for $name {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                f.write_str(self.as_str())
            }
        }

        impl ::std::str::FromStr for $name {
            type Err = ::std::convert::Infallible;
            fn from_str(s: &str) -> ::std::result::Result<Self, Self::Err> {
                Ok(match s {
                    $($value => Self::$variant,)+
                    other => Self::Unknown(other.to_owned()),
                })
            }
        }

        impl From<&str> for $name {
            fn from(s: &str) -> Self {
                match s.parse() {
                    Ok(v) => v,
                    Err(never) => match never {},
                }
            }
        }

        impl ::serde::Serialize for $name {
            fn serialize<S: ::serde::Serializer>(&self, s: S) -> ::std::result::Result<S::Ok, S::Error> {
                s.serialize_str(self.as_str())
            }
        }

        impl<'de> ::serde::Deserialize<'de> for $name {
            fn deserialize<D: ::serde::Deserializer<'de>>(d: D) -> ::std::result::Result<Self, D::Error> {
                let s = <::std::borrow::Cow<'de, str> as ::serde::Deserialize>::deserialize(d)?;
                Ok(Self::from(s.as_ref()))
            }
        }
    };
}
