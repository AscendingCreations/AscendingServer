use crate::{gametypes::*, network::*};
use log::{error, trace, warn};
use mmap_bytey::{MByteBuffer, BUFFER_SIZE};
use tokio::{net::tcp::OwnedReadHalf, sync::mpsc};

pub struct SocketActor {
    pub rx: OwnedReadHalf,
    pub tx: mpsc::Sender<ClientPacket>,
    pub buffer: ByteBuffer,
    pub addr: Arc<String>,
}

impl SocketActor {
    pub fn new(
        rx: OwnedReadHalf,
        tx: mpsc::Sender<ClientPacket>,
        addr: Arc<String>,
    ) -> Result<Self> {
        Ok(Self {
            rx,
            tx,
            buffer: ByteBuffer::with_capacity(8192)?,
            addr,
        })
    }

    pub async fn process(mut self) -> Result<()> {
        let mut buf: [u8; 4096] = [0; 4096];
        let mut packet = MByteBuffer::new().unwrap();

        loop {
            match self.rx.try_read(&mut buf) {
                Err(ref err) if err.kind() == io::ErrorKind::WouldBlock => {}
                Err(ref err) if err.kind() == io::ErrorKind::Interrupted => {}
                Ok(0) => return self.disconnect().await,
                Err(e) => {
                    trace!("stream.read, error in socket read: {}", e);
                    return self.disconnect().await;
                }
                Ok(n) => {
                    let pos = self.buffer.cursor();
                    self.buffer.move_cursor_to_end();

                    if let Err(e) = self.buffer.write_slice(&buf[0..size]) {
                        trace!(
                            "buffer.write_slice, error in socket read: {} for addr: {}",
                            e,
                            &*self.addr
                        );
                        return self.disconnect().await;
                    }

                    if let Err(e) = self.buffer.move_cursor(pos) {
                        trace!("buffer move_cursor. error: {}", e);
                        return self.disconnect().await;
                    }
                }
            };

            loop {
                packet.move_cursor_to_start();

                let length = match self.get_length().await {
                    Ok(n) => n,
                    Err(e) => {
                        error!("{}", e);
                        return self.disconnect().await;
                    }
                };

                if let Some(length) = length {
                    if length <= (self.buffer.length() - self.buffer.cursor()) as u64 {
                        let mut errored = false;

                        if let Ok(bytes) = self.buffer.read_slice(length as usize) {
                            if packet.write_slice(bytes).is_err() {
                                errored = true;
                            }

                            packet.move_cursor_to_start();
                        } else {
                            errored = true;
                        }

                        if errored {
                            warn!(
                                "IP: {} was disconnected due to error on packet processing.",
                                self.addr
                            );
                            return self.disconnect().await;
                        }

                        let processed_packet = match packet_translator(data) {
                            Ok(v) => v,
                            Err(e) => {
                                error!("{}", e);
                                return self.disconnect().await;
                            }
                        };

                        self.tx.send(processed_packet).await?;
                    } else {
                        break;
                    }
                } else {
                    break;
                }
            }
            // Move cursor back from when we got length.
            let cursor = self.buffer.cursor() - 8;

            if let Err(e) = self.buffer.move_cursor(cursor) {
                error!("{}", e);
                return self.disconnect().await;
            }

            let buffer_len = self.buffer.length() - cursor;

            if cursor == self.buffer.length() {
                if let Err(e) = self.buffer.truncate(0) {
                    error!("{}", e);
                    return self.disconnect().await;
                }

                if self.buffer.capacity() > 500000 {
                    warn!(
                        "process_packets: buffer resize to 100000. Buffer Capacity: {}, Buffer len: {}",
                        self.buffer.capacity(),
                        buffer_len
                    );

                    if let Err(e) = self.buffer.resize(100000) {
                        error!("{}", e);
                        return self.disconnect().await;
                    }
                }
            } else if self.buffer.capacity() > 500000 && buffer_len <= 100000 {
                warn!(
                    "process_packets: buffer resize to Buffer len. Buffer Capacity: {}, Buffer len: {}",
                    self.buffer.capacity(),
                    buffer_len
                );

                let mut replacement = match ByteBuffer::with_capacity(buffer_len) {
                    Ok(v) => v,
                    Err(e) => {
                        error!("{}", e);
                        return self.disconnect().await;
                    }
                };

                let slice = match self.buffer.read_slice(buffer_len) {
                    Ok(v) => v,
                    Err(e) => {
                        error!("{}", e);
                        return self.disconnect().await;
                    }
                };

                if let Err(e) = replacement.write_slice(slice) {
                    error!("{}", e);
                    return self.disconnect().await;
                }

                replacement.move_cursor_to_start();

                self.buffer = replacement;
            }
        }
    }

    pub async fn disconnect(&self) -> Result<()> {
        trace!("Players Disconnected IP: {} ", &self.addr);
        self.tx.send(ClientPacket::Disconnect).await?;
        Ok(())
    }

    pub async fn get_length(&mut self) -> Result<Option<u64>> {
        if self.buffer.length() - self.buffer.cursor() >= 8 {
            let length = self.buffer.read::<u64>()?;

            if !(1..=BUFFER_SIZE as u64).contains(&length) {
                return Err(AscendingError::InvalidPacketSize {
                    length,
                    addr: self.addr.clone(),
                    max: BUFFER_SIZE,
                });
            }

            return Ok(Some(length));
        }

        Ok(None)
    }
}
