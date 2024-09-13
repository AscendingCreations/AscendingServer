use crate::{containers::*, gametypes::*, ipc::*};
use bytey::ByteBuffer;
use interprocess::local_socket::tokio::{prelude::*, SendHalf, Stream};
use mmap_bytey::{MByteBuffer, BUFFER_SIZE};
use sqlx::PgPool;
use std::sync::{atomic::AtomicU64, Arc};
use tokio::{
    io::{AsyncReadExt, BufReader},
    sync::mpsc,
};

pub struct IPCActor {
    pub info_tx: mpsc::Sender<InfoIncomming>,
    pub npc_count: Arc<AtomicU64>,
    pub player_count: Arc<AtomicU64>,
    pub pgconn: PgPool,
    pub bases: Arc<Bases>,
    pub config: Arc<Config>,
}

impl IPCActor {
    pub fn new(storage: &Storage, info_tx: mpsc::Sender<InfoIncomming>) -> Self {
        Self {
            info_tx,
            npc_count: storage.npc_count.clone(),
            player_count: storage.player_count.clone(),
            pgconn: storage.pgconn.clone(),
            bases: storage.bases.clone(),
            config: storage.config.clone(),
        }
    }

    pub async fn runner(self, conn: Stream) -> Result<()> {
        let (rx, mut tx) = conn.split();
        let mut rx = BufReader::new(&rx);
        let mut buffer = ByteBuffer::new()?;

        // Allocate a sizeable buffer for receiving. This size should be big enough and easy to
        // find for the allocator.
        let mut buf: [u8; 4096] = [0; 4096];
        let mut packet = MByteBuffer::new().unwrap();

        loop {
            match rx.read(&mut buf).await {
                Err(e) => {
                    log::error!("handle_conn, error in ipc read: {}", e);
                    return Ok(());
                }
                Ok(size) => {
                    let pos = buffer.cursor();
                    buffer.move_cursor_to_end();

                    if let Err(e) = buffer.write_slice(&buf[0..size]) {
                        log::error!("buffer.write_slice, error in socket read: {}", e);
                        return Ok(());
                    }

                    if let Err(e) = buffer.move_cursor(pos) {
                        log::trace!("buffer move_cursor. error: {}", e);
                        return Ok(());
                    }
                }
            };

            loop {
                packet.move_cursor_to_start();

                let length = match self.get_length(&mut buffer).await {
                    Ok(n) => n,
                    Err(e) => {
                        log::error!("{}", e);
                        return Ok(());
                    }
                };

                if let Some(length) = length {
                    if length <= (buffer.length() - buffer.cursor()) as u64 {
                        let mut errored = false;

                        if let Ok(bytes) = buffer.read_slice(length as usize) {
                            if packet.write_slice(bytes).is_err() {
                                errored = true;
                            }

                            packet.move_cursor_to_start();
                        } else {
                            errored = true;
                        }

                        if errored {
                            log::warn!("ipc disconnected due to error on packet processing.");
                            return Ok(());
                        }

                        match self.packet_translator(&mut packet, &mut tx).await {
                            Ok(v) => v,
                            Err(e) => {
                                log::error!("{}", e);
                                return Ok(());
                            }
                        };
                    } else {
                        break;
                    }
                } else {
                    break;
                }
            }
            // Move cursor back from when we got length.
            let cursor = buffer.cursor() - 8;

            if let Err(e) = buffer.move_cursor(cursor) {
                log::error!("{}", e);
                return Ok(());
            }

            let buffer_len = buffer.length() - cursor;

            if cursor == buffer.length() {
                if let Err(e) = buffer.truncate(0) {
                    log::error!("{}", e);
                    return Ok(());
                }

                if buffer.capacity() > 500000 {
                    log::warn!(
                        "process_packets: buffer resize to 100000. Buffer Capacity: {}, Buffer len: {}",
                        buffer.capacity(),
                        buffer_len
                    );

                    if let Err(e) = buffer.resize(100000) {
                        log::error!("{}", e);
                        return Ok(());
                    }
                }
            } else if buffer.capacity() > 500000 && buffer_len <= 100000 {
                log::warn!(
                    "process_packets: buffer resize to Buffer len. Buffer Capacity: {}, Buffer len: {}",
                    buffer.capacity(),
                    buffer_len
                );

                let mut replacement = match ByteBuffer::with_capacity(buffer_len) {
                    Ok(v) => v,
                    Err(e) => {
                        log::error!("{}", e);
                        return Ok(());
                    }
                };

                let slice = match buffer.read_slice(buffer_len) {
                    Ok(v) => v,
                    Err(e) => {
                        log::error!("{}", e);
                        return Ok(());
                    }
                };

                if let Err(e) = replacement.write_slice(slice) {
                    log::error!("{}", e);
                    return Ok(());
                }

                replacement.move_cursor_to_start();

                buffer = replacement;
            }
        }
    }

    pub async fn get_length(&self, buffer: &mut ByteBuffer) -> Result<Option<u64>> {
        if buffer.length() - buffer.cursor() >= 8 {
            let length = buffer.read::<u64>()?;

            if !(1..=BUFFER_SIZE as u64).contains(&length) {
                return Err(AscendingError::InvalidPacketSize {
                    length,
                    addr: Arc::new("IPC".to_owned()),
                    max: BUFFER_SIZE,
                });
            }

            return Ok(Some(length));
        }

        Ok(None)
    }

    pub async fn packet_translator(&self, data: &mut MByteBuffer, tx: &mut SendHalf) -> Result<()> {
        let id: ClientIPCID = data.read()?;

        match id {
            ClientIPCID::GetUserList => handle_get_users(self, data, tx).await,
        }
    }
}
