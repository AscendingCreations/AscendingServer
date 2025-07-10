#![allow(dead_code)]
#![recursion_limit = "256"]
#![feature(error_generic_member_access)]

mod containers;
mod gameloop;
mod gametypes;
mod items;
mod maps;
mod npcs;
mod players;
mod socket;
mod sql;
mod tasks;
mod time_ext;

#[allow(unused_imports)]
use backtrace::Backtrace;
use containers::{Storage, World};
use gameloop::*;
use gametypes::*;
use log::{Level, Metadata, Record, error, info};
use std::{env, fs::File, io::Write, panic};

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

fn main() {
    let config = read_config("settings.toml");
    log::set_logger(&MY_LOGGER).unwrap();
    // Set the Max level we accept logging to the file for.
    log::set_max_level(config.level_filter.parse_enum());

    if config.enable_backtrace {
        unsafe { env::set_var("RUST_BACKTRACE", "1") };
    }

    panic::set_hook(Box::new(|panic_info| {
        let bt = Backtrace::new();

        error!(
            "::::::::PANIC::::::::\n 
            {panic_info}\n
            :::::::::::::::::::::\n
            ::::::BACKTRACE::::::\n
            {bt:?}\n
            :::::::::::::::::::::\n"
        );
    }));

    info!("Starting up");
    info!("Initializing Storage");
    let storage = Storage::new(config).unwrap();
    info!("Initializing PacketRouter");
    let router = PacketRouter::init();
    info!("Initializing World");
    let mut world = World::default();

    info!("Game Server is Running.");
    game_loop(&mut world, &storage, &router);
}
