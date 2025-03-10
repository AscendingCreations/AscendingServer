use crate::{gametypes::MapPosition, tasks::DataTaskToken};
use std::{
    backtrace::Backtrace,
    sync::{PoisonError, TryLockError},
};
use thiserror::Error;

pub type Result<T> = std::result::Result<T, AscendingError>;

#[derive(Error, Debug)]
pub enum AscendingError {
    #[error("Unknown Error Occured. Backtrace: {0}")]
    Unhandled(Box<Backtrace>),
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
    #[error("Error: {error}, BackTrace: {backtrace}")]
    AddrParseError {
        #[from]
        error: std::net::AddrParseError,
        #[backtrace]
        backtrace: Box<Backtrace>,
    },
    #[error("Error: {error}, BackTrace: {backtrace}")]
    Io {
        #[from]
        error: std::io::Error,
        #[backtrace]
        backtrace: Box<Backtrace>,
    },
    #[error("Error: {error}, BackTrace: {backtrace}")]
    UnicodeError {
        #[from]
        error: std::str::Utf8Error,
        #[backtrace]
        backtrace: Box<Backtrace>,
    },
    #[error("Error: {error}, BackTrace: {backtrace}")]
    ByteyError {
        #[from]
        error: bytey::ByteBufferError,
        #[backtrace]
        backtrace: Box<Backtrace>,
    },
    #[error("Error: {error}, BackTrace: {backtrace}")]
    MByteyError {
        #[from]
        error: mmap_bytey::MByteBufferError,
        #[backtrace]
        backtrace: Box<Backtrace>,
    },
    #[error("Error: {error}, BackTrace: {backtrace}")]
    RegexError {
        #[from]
        error: regex::Error,
        #[backtrace]
        backtrace: Box<Backtrace>,
    },
    #[error("Error: {error}, BackTrace: {backtrace}")]
    ParseError {
        #[from]
        error: std::string::ParseError,
        #[backtrace]
        backtrace: Box<Backtrace>,
    },
    #[error("Error: {error}, BackTrace: {backtrace}")]
    Sqlx {
        #[from]
        error: sqlx::Error,
        #[backtrace]
        backtrace: Box<Backtrace>,
    },
    #[error("Error: {error}, BackTrace: {backtrace}")]
    Rustls {
        #[from]
        error: rustls::Error,
        #[backtrace]
        backtrace: Box<Backtrace>,
    },
    #[error("Error: {error}, BackTrace: {backtrace}")]
    TomlDe {
        #[from]
        error: toml::de::Error,
        #[backtrace]
        backtrace: Box<Backtrace>,
    },
    #[error("Error: {error}, BackTrace: {backtrace}")]
    RustlsVerifierBuilder {
        #[from]
        error: rustls::client::VerifierBuilderError,
        #[backtrace]
        backtrace: Box<Backtrace>,
    },
    #[error("Mutex PoisonError Occured, BackTrace: {backtrace}")]
    MutexLockError {
        #[backtrace]
        backtrace: Box<Backtrace>,
    },
    #[error("GlobalKey Kind is Missing Does it even Exist?, BackTrace: {backtrace}")]
    MissingKind {
        #[backtrace]
        backtrace: Box<Backtrace>,
    },
    #[error("GlobalKey is Missing, BackTrace: {backtrace}")]
    MissingEntity {
        #[backtrace]
        backtrace: Box<Backtrace>,
    },
    #[error("TryLock Error, BackTrace: {backtrace}")]
    TryLockError {
        #[backtrace]
        backtrace: Box<Backtrace>,
    },
}

impl<T> From<TryLockError<T>> for AscendingError {
    fn from(_: TryLockError<T>) -> Self {
        Self::TryLockError {
            backtrace: Box::new(Backtrace::capture()),
        }
    }
}

impl<T> From<PoisonError<T>> for AscendingError {
    fn from(_: PoisonError<T>) -> Self {
        Self::MutexLockError {
            backtrace: Box::new(Backtrace::capture()),
        }
    }
}

impl AscendingError {
    pub fn missing_kind() -> Self {
        AscendingError::MissingKind {
            backtrace: Box::new(Backtrace::capture()),
        }
    }

    pub fn missing_entity() -> Self {
        AscendingError::MissingEntity {
            backtrace: Box::new(Backtrace::capture()),
        }
    }
}
