use crate::{containers::*, gametypes::*, ipc::*};
use interprocess::local_socket::{tokio::prelude::*, GenericNamespaced, ListenerOptions};

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

        let ipc_actor = IPCActor::new(storage, info_tx.clone());

        tokio::spawn(async move {
            if let Err(e) = ipc_actor.runner(conn).await {
                log::error!("Error while handling connection: {}", e);
            }
        });
    }
}
