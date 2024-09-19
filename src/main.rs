#![allow(dead_code, clippy::let_and_return)]
#![recursion_limit = "256"]
#![feature(let_chains, error_generic_member_access)]
#![feature(async_closure)]

mod containers;
mod gameloop;
mod gametypes;
mod identity;
mod ipc;
mod items;
mod logins;
mod maps;
mod network;
mod npcs;
mod players;
mod sql;
mod tasks;
mod time_ext;

#[allow(unused_imports)]
use backtrace::Backtrace;
use containers::Storage;
use gametypes::*;
use identity::*;
use ipc::ipc_runner;
use log::{error, info, Level, Metadata, Record};
use logins::LoginIncomming;
use std::{env, fs::File, io::Write, panic};
use tokio::sync::mpsc;

use crate::containers::read_config;

// used to get string input when we add a command console to control the game.
// until then we will just not use this.
fn read_line() -> String {
    let mut rv = String::new();
    std::io::stdin().read_line(&mut rv).unwrap();
    rv.replace("\r\n", "").replace('\n', "")
}

// creates a static global logger type for setting the logger
static MY_LOGGER: MyLogger = MyLogger(Level::Debug);

struct MyLogger(pub Level);

impl log::Log for MyLogger {
    // checks if it can log these types of events.
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= self.0
    }

    // This logs to a panic file. This is so we can see
    // Errors and such if a program crashes in full render mode.
    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            let msg = format!("{} - {}\n", record.level(), record.args());
            println!("{}", &msg);

            let mut file = match File::options()
                .append(true)
                .create(true)
                .open("ServerLog.txt")
            {
                Ok(v) => v,
                Err(_) => return,
            };

            let _ = file.write(msg.as_bytes());
        } else if self.0 == Level::Info {
            let msg = format!("{} - {}\n", record.level(), record.args());
            println!("{}", &msg);
        }
    }
    fn flush(&self) {}
}

#[tokio::main]
async fn main() {
    let config = read_config("settings.toml");
    //we do this to ensure all ports are different.
    assert_ne!(
        config.listen_port, config.login_server_port,
        "Listen Port and Login server port can not be the same."
    );
    assert_ne!(
        config.listen_port, config.login_server_secure_port,
        "Listen Port and Login server secure ports can not be the same."
    );
    assert_ne!(
        config.login_server_port, config.login_server_secure_port,
        "Login server port and Login server secure ports can not be the same."
    );
    //console_subscriber::init();

    log::set_logger(&MY_LOGGER).unwrap();
    // Set the Max level we accept logging to the file for.
    log::set_max_level(config.level_filter.parse_enum());

    if config.enable_backtrace {
        env::set_var("RUST_BACKTRACE", "1");
    }

    panic::set_hook(Box::new(|panic_info| {
        let bt = Backtrace::new();

        error!(
            "::::::::PANIC::::::::\n 
            {}\n
            :::::::::::::::::::::\n
            ::::::BACKTRACE::::::\n
            {:?}\n
            :::::::::::::::::::::\n",
            panic_info, bt
        );
    }));

    info!("Starting up");
    let (id_tx, id_rx) = mpsc::channel::<IDIncomming>(config.map_buffer_size);

    info!("Initializing Storage Data");
    let mut storage = Storage::new(config, id_tx).await.unwrap();

    info!("Initializing World");
    storage.generate_world_actors().await.unwrap();

    info!("Initializing Game Time Actor");
    let time_actor = GameTimeActor::new(storage.map_broadcast_tx.clone());
    tokio::spawn(time_actor.runner());

    let id_actor = IDActor::new(&storage, id_rx);
    tokio::spawn(id_actor.runner());

    info!("Initializing Login Channels");
    let (login_tx, login_rx) = mpsc::channel::<LoginIncomming>(1000);

    info!("Initializing Login Actor");
    let login_actor = logins::LoginActor::new(&storage, login_rx).await;
    tokio::spawn(login_actor.runner());

    info!("Initializing Server Listener Actor");
    let listening_actor = network::ServerListenActor::new(&storage, login_tx.clone())
        .await
        .unwrap();
    tokio::spawn(listening_actor.runner());
    info!("Game Server is Running.");

    ipc_runner(&storage, login_tx).await.unwrap();
}
