use crate::{containers::*, gametypes::*, ipc::*, maps::*};
use bytey::ByteBuffer;
use interprocess::local_socket::{
    tokio::{prelude::*, RecvHalf, SendHalf, Stream},
    GenericNamespaced, ListenerOptions,
};
use mmap_bytey::{MByteBuffer, BUFFER_SIZE};
use sqlx::PgPool;
use std::{
    io::{Read, Write},
    sync::{atomic::AtomicU64, Arc},
};
use tokio::{
    // io::{AsyncReadExt, AsyncWriteExt, BufReader},
    sync::{broadcast, mpsc},
};

pub struct IPCActor {
    pub conn: Stream,
    pub npc_count: Arc<AtomicU64>,
    pub player_count: Arc<AtomicU64>,
    pub map_senders: IndexMap<MapPosition, mpsc::Sender<MapIncomming>>,
    pub map_broadcast_tx: broadcast::Sender<MapBroadCasts>,
    pub map_broadcast_rx: broadcast::Sender<MapBroadCasts>,
    pub pgconn: PgPool,
    pub bases: Arc<Bases>,
    pub config: Arc<Config>,
}

async fn handle_conn(conn: Stream) -> Result<()> {
    let (mut rx, mut tx) = conn.split();
    let mut buffer = ByteBuffer::new();

    // Allocate a sizeable buffer for receiving. This size should be big enough and easy to
    // find for the allocator.
    let mut buf: [u8; 4096] = [0; 4096];

    // Describe the send operation as sending our whole message.
    //let send = tx.write_all(b"Hello from server!\n");
    // Describe the receive operation as receiving a line into our big buffer.
    let recv = rx.read(&mut buf);

    match recv {
        Ok(v) => {}
        Err(e) => {}
    };

    // Produce our output!
    //println!("Client answered: {}", buffer.trim());
    Ok(())
}

pub async fn ipc_runner(storage: &Storage) -> Result<()> {
    let name = (*storage.config.ipc_name).to_ns_name::<GenericNamespaced>()?;
    let opts = ListenerOptions::new().name(name);

    let (info_tx, actor) = InfoActor::new(storage.map_broadcast_tx.subscribe());

    log::info!("Initializing Info Actor");
    tokio::spawn(actor.runner());

    let listener = match opts.create_tokio() {
        Err(e) if e.kind() == std::io::ErrorKind::AddrInUse => {
            log::error!(
				"
                    Error: could not start server because the socket file is occupied. Please check if {}
                    is in use by another process and try again.", storage.config.ipc_name
			);
            return Err(e.into());
        }
        Err(e) => {
            log::error!("ipc runner failed with error: {}", e);
            return Err(e.into());
        }
        Ok(v) => v,
    };

    log::info!("Server running at {}", storage.config.ipc_name);

    loop {
        let conn = match listener.accept().await {
            Ok(c) => c,
            Err(e) => {
                log::error!("There was an error with an incoming connection: {e}");
                continue;
            }
        };

        tokio::spawn(async move {
            if let Err(e) = handle_conn(conn).await {
                eprintln!("Error while handling connection: {e}");
            }
        });
    }
}
