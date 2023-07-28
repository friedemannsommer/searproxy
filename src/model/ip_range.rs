use std::{
    fmt::{Display, Formatter},
    str::FromStr,
};

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub enum PermittedIpRange {
    None,
    Global,
    Private,
    Local,
}

#[derive(thiserror::Error, Debug)]
pub enum IpRangeParseError {
    #[error("Unknown IP range variant `{0}`")]
    Unrecognized(String),
}

impl Display for PermittedIpRange {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::None => f.write_str("none"),
            Self::Global => f.write_str("global"),
            Self::Private => f.write_str("private"),
            Self::Local => f.write_str("local"),
        }
    }
}

impl FromStr for PermittedIpRange {
    type Err = IpRangeParseError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Ok(match value {
            "none" => PermittedIpRange::None,
            "local" => PermittedIpRange::Local,
            "private" => PermittedIpRange::Private,
            "global" => PermittedIpRange::Global,
            _ => return Err(IpRangeParseError::Unrecognized(String::from(value))),
        })
    }
}
