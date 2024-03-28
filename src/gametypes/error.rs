use crate::{gametypes::MapPosition, tasks::DataTaskToken};
use thiserror::Error;

pub type Result<T> = std::result::Result<T, AscendingError>;

#[derive(Error, Debug)]
pub enum AscendingError {
    #[error("Currently Unhandled data error")]
    Unhandled,
    #[error("Multiple Logins Detected")]
    MultiLogin,
    #[error("Failed to register account")]
    RegisterFail,
    #[error("Failed to find the user account")]
    UserNotFound,
    #[error("Attempted usage of Socket when connection was not accepted")]
    InvalidSocket,
    #[error("Packet Manipulation detect from {name}")]
    PacketManipulation { name: String },
    #[error("Failed Packet Handling at {num} with message: {message}")]
    PacketReject { num: usize, message: String },
    #[error("Packet id was invalid")]
    InvalidPacket,
    #[error("Password was incorrect")]
    IncorrectPassword,
    #[error("No username was set.")]
    NoUsernameSet,
    #[error("No password was set")]
    NoPasswordSet,
    #[error("Map at Position {0:?} not found")]
    MapNotFound(MapPosition),
    #[error("NPC ID {0:?} not found")]
    NpcNotFound(u64),
    #[error("Packet buffer {0:?} not found")]
    PacketCacheNotFound(DataTaskToken),
    #[error(transparent)]
    AddrParseError(#[from] std::net::AddrParseError),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    UnicodeError(#[from] std::str::Utf8Error),
    #[error(transparent)]
    ByteyError(#[from] bytey::ByteBufferError),
    #[error(transparent)]
    RegexError(#[from] regex::Error),
    #[error(transparent)]
    ParseError(#[from] std::string::ParseError),
    #[error(transparent)]
    HecNoEntity(#[from] hecs::NoSuchEntity),
    #[error(transparent)]
    Sqlx(#[from] sqlx::Error),
    #[error(transparent)]
    HecsComponent(#[from] hecs::ComponentError),
    #[error(transparent)]
    HecsQueryOne(#[from] hecs::QueryOneError),
    #[error(transparent)]
    Rustls(#[from] rustls::Error),
    #[error(transparent)]
    TomlDe(#[from] toml::de::Error),
}
