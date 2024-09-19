use std::{net::SocketAddr, str::FromStr, sync::Arc};

use crate::{containers::Storage, gametypes::Result, network::Socket, AscendingError};

use bytey::ByteBuffer;
use log::warn;
use mmap_bytey::{MByteBuffer, BUFFER_SIZE};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{tcp::OwnedReadHalf, TcpSocket, TcpStream},
    sync::mpsc,
};

pub enum LoginIncomming {
    GetSocket(Socket),
    UpdateLoginServer { users_len: u64 },
}

pub struct LoginActor {
    pub login_rx: mpsc::Receiver<LoginIncomming>,
    pub login_stream: TcpStream,
    pub storage: Storage,
}

/* let mut packet = MByteBuffer::new_packet().unwrap();
packet.write(&self.config.server_name).unwrap();
packet.write(&self.config.listen_ip).unwrap();
packet.write(self.config.listen_port).unwrap();
packet.write(self.usernames.len() as u64).unwrap();
packet.write(self.config.max_players).unwrap();
packet.finish().unwrap()

self.login_stream.write_all(src)
*/

impl LoginActor {
    pub async fn new(storage: &Storage, login_rx: mpsc::Receiver<LoginIncomming>) -> Self {
        let addr = SocketAddr::new(
            std::net::IpAddr::from_str(&storage.config.login_server_ip).unwrap(),
            storage.config.login_server_port,
        );
        let socket = TcpSocket::new_v4().unwrap();
        let login_stream = socket.connect(addr).await.unwrap();

        login_stream.set_nodelay(true).unwrap();

        Self {
            login_rx,
            login_stream,
            storage: storage.clone(),
        }
    }

    pub async fn runner(mut self) -> Result<()> {
        let mut login_buffer = ByteBuffer::new()?;
        let (mut login_socket_rx, mut login_socket_tx) = self.login_stream.into_split();
        let mut clients: Vec<Socket> = Vec::new();

        loop {
            let packet = login_socket_read(&mut login_socket_rx, &mut login_buffer).await?;
        }

        Ok(())
    }
}

pub async fn login_socket_read(
    rx: &mut OwnedReadHalf,
    buffer: &mut ByteBuffer,
) -> Result<MByteBuffer> {
    let mut buf: [u8; 4096] = [0; 4096];

    loop {
        let size = rx.read(&mut buf).await?;
        let pos = buffer.cursor();

        buffer.move_cursor_to_end();
        buffer.write_slice(&buf[0..size])?;
        buffer.move_cursor(pos)?;

        let length = get_length(buffer)?;

        if let Some(length) = length {
            if length <= (buffer.length() - buffer.cursor()) as u64 {
                let mut packet = MByteBuffer::new()?;
                let slice = buffer.read_slice(length as usize)?;
                packet.write_slice(slice)?;
                packet.move_cursor_to_start();

                buffer_check(buffer)?;
                return Ok(packet);
            }
        }

        // Move cursor back from when we got length.
        let cursor = buffer.cursor() - 8;
        buffer.move_cursor(cursor)?;
    }
}

pub fn get_length(buffer: &mut ByteBuffer) -> Result<Option<u64>> {
    if buffer.length() - buffer.cursor() >= 8 {
        let length = buffer.read::<u64>()?;

        if !(1..=BUFFER_SIZE as u64).contains(&length) {
            return Err(AscendingError::InvalidPacketSize {
                length,
                addr: Arc::new("Login".to_owned()),
                max: BUFFER_SIZE,
            });
        }

        return Ok(Some(length));
    }

    Ok(None)
}

pub fn buffer_check(buffer: &mut ByteBuffer) -> Result<()> {
    let buffer_len = buffer.length() - buffer.cursor();

    if buffer.cursor() == buffer.length() {
        buffer.truncate(0)?;

        if buffer.capacity() > 500000 {
            warn!(
                "Login: buffer resize to 100000. Buffer Capacity: {}, Buffer len: {}",
                buffer.capacity(),
                buffer_len
            );

            buffer.resize(100000)?;
        }
    } else if buffer.capacity() > 500000 && buffer_len <= 100000 {
        warn!(
            "Login: buffer resize to Buffer len. Buffer Capacity: {}, Buffer len: {}",
            buffer.capacity(),
            buffer_len
        );

        let mut replacement = ByteBuffer::with_capacity(buffer_len)?;

        replacement.write_slice(buffer.read_slice(buffer_len)?)?;
        replacement.move_cursor_to_start();
        *buffer = replacement;
    }

    Ok(())
}
