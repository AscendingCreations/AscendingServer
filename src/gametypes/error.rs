use crate::gametypes::MapPosition;
use std::{backtrace::Backtrace, sync::Arc};
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
    #[error(
        "Packet Length was too small or too big! Length {length} of limit 1..{max}, addr: {addr}"
    )]
    InvalidPacketSize {
        length: u64,
        addr: Arc<String>,
        max: usize,
    },
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
    // #[error("Packet buffer {0:?} not found")]
    //PacketCacheNotFound(DataTaskToken),
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
    HecNoEntity {
        #[from]
        error: hecs::NoSuchEntity,
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
    HecsComponent {
        #[from]
        error: hecs::ComponentError,
        #[backtrace]
        backtrace: Box<Backtrace>,
    },
    #[error("Error: {error}, BackTrace: {backtrace}")]
    HecsQueryOne {
        #[from]
        error: hecs::QueryOneError,
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
    #[error("Error: {error}, BackTrace: {backtrace}")]
    TokioMPSCLoginSendError {
        #[from]
        error: Box<tokio::sync::mpsc::error::SendError<crate::logins::LoginIncomming>>,
        #[backtrace]
        backtrace: Box<Backtrace>,
    },
    #[error("Error: {error}, BackTrace: {backtrace}")]
    TokioMPSCPlayerSendError {
        #[from]
        error: Box<tokio::sync::mpsc::error::SendError<crate::network::ClientPacket>>,
        #[backtrace]
        backtrace: Box<Backtrace>,
    },
    #[error("Error: {error}, BackTrace: {backtrace}")]
    TokioBroadcastMapSendError {
        #[from]
        error: Box<tokio::sync::broadcast::error::SendError<crate::maps::MapBroadCasts>>,
        #[backtrace]
        backtrace: Box<Backtrace>,
    },
    #[error("Error: {error}, BackTrace: {backtrace}")]
    TokioOneShotInfoRecvError {
        #[from]
        error: tokio::sync::oneshot::error::RecvError,
        #[backtrace]
        backtrace: Box<Backtrace>,
    },
    #[error("Error: {error}, BackTrace: {backtrace}")]
    TokioMPSCInfoSendError {
        #[from]
        error: Box<tokio::sync::mpsc::error::SendError<crate::ipc::InfoIncomming>>,
        #[backtrace]
        backtrace: Box<Backtrace>,
    },
    #[error("Error: {error}, BackTrace: {backtrace}")]
    TokioMPSCRecvTryError {
        #[from]
        error: tokio::sync::mpsc::error::TryRecvError,
        #[backtrace]
        backtrace: Box<Backtrace>,
    },
    #[error("Error: {error}, BackTrace: {backtrace}")]
    TokioBroadcastRecvError {
        #[from]
        error: tokio::sync::broadcast::error::RecvError,
        #[backtrace]
        backtrace: Box<Backtrace>,
    },
}
