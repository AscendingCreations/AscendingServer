use super::ClientPacket;
use crate::{gametypes::Result, AscendingError};
use log::trace;
use mmap_bytey::MByteBuffer;
use std::{backtrace::Backtrace, sync::Arc};
use tokio::{io::AsyncWriteExt, net::tcp::OwnedWriteHalf, sync::mpsc};

pub struct Socket {
    pub tx: OwnedWriteHalf,
    pub rx: mpsc::Receiver<ClientPacket>,
    pub addr: Arc<String>,
}

impl Socket {
    pub fn new(tx: OwnedWriteHalf, rx: mpsc::Receiver<ClientPacket>, addr: Arc<String>) -> Self {
        Self { tx, rx, addr }
    }

    pub async fn send(&mut self, packet: &mut MByteBuffer) -> Result<()> {
        if let Err(error) = self.tx.write_all(packet.as_slice()).await {
            trace!("Send error in socket write: {}", error);
            return Err(AscendingError::Io {
                error,
                backtrace: Box::new(Backtrace::capture()),
            });
        }

        Ok(())
    }
}

/*
#[inline]
pub async fn send_to(storage: &GameStore, socket_id: usize, buf: MByteBuffer) -> Result<()> {
    if let Some(client) = storage
        .server
        .read()
        .await
        .clients
        .get(&mio::Token(socket_id))
    {
        client.lock().await.send(&*storage.poll.read().await, buf)
    } else {
        Ok(())
    }
}

#[inline]
pub async fn send_to_all(world: &GameWorld, storage: &GameStore, buf: MByteBuffer) -> Result<()> {
    let lock = world.read().await;
    for (_entity, (_, socket)) in lock
        .query::<((&WorldEntityType, &OnlineType), &Socket)>()
        .iter()
        .filter(|(_entity, ((worldentitytype, onlinetype), _))| {
            **worldentitytype == WorldEntityType::Player && **onlinetype == OnlineType::Online
        })
    {
        if let Some(client) = storage
            .server
            .read()
            .await
            .clients
            .get(&mio::Token(socket.id))
        {
            client
                .lock()
                .await
                .send(&*storage.poll.read().await, buf.try_clone()?)?;
        }
    }

    Ok(())
}

#[inline]
pub async fn send_to_maps(
    world: &GameWorld,
    storage: &GameStore,
    position: MapPosition,
    buf: MByteBuffer,
    avoidindex: Option<Entity>,
) -> Result<()> {
    for m in get_surrounding(position, true) {
        let map = match storage.maps.get(&m) {
            Some(map) => map,
            None => continue,
        }
        .read()
        .await;

        for entity in &map.players {
            if avoidindex.map(|value| value == *entity).unwrap_or(false) {
                continue;
            }

            let lock = world.read().await;
            let mut query = lock.query_one::<(&OnlineType, &Socket)>(entity.0)?;

            if let Some((status, socket)) = query.get() {
                if *status == OnlineType::Online {
                    if let Some(client) = storage
                        .server
                        .read()
                        .await
                        .clients
                        .get(&mio::Token(socket.id))
                    {
                        client
                            .lock()
                            .await
                            .send(&*storage.poll.read().await, buf.try_clone()?)?;
                    }
                }
            }
        }
    }

    Ok(())
}*/
