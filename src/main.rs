#![allow(dead_code)]
#![recursion_limit = "256"]
#![feature(test, let_chains)]
extern crate test;
#[macro_use]
extern crate derivative;

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
mod database;

use std::{fs::File, io::Write};

#[allow(unused_imports)]
use backtrace::Backtrace;
use gameloop::*;
use gametypes::*;
use log::{info, Level, LevelFilter, Metadata, Record};
//use npcs::*;
//use player::Player;
//use serde::{Deserialize, Serialize};
//use socket::*;
//use sql::*;
use containers::Storage;
//use test::Bencher;
//use bytey::ByteBuffer;
//use bytey::{ByteBufferRead, ByteBufferWrite};
//use time_ext::{MyDuration, MyInstant};
use hecs::World;

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
        }
    }
    fn flush(&self) {}
}

fn main() {
    log::set_logger(&MY_LOGGER).unwrap();
    // Set the Max level we accept logging to the file for.
    log::set_max_level(LevelFilter::Info);

    info!("Starting up");
    info!("Initializing Storage");
    let storage = match Storage::new() {
        Some(n) => n,
        None => return,
    };
    info!("Initializing PacketRouter");
    let router = PacketRouter::init();
    info!("Initializing World");
    let mut world = World::new();

    game_loop(&mut world, &storage, &router);
    // So we get a log and it displays in the command line.
    info!("Server ShutDown Completed. Press Enter to Exit Program.");
    println!("Server ShutDown Completed. Press Enter to Exit Program.");

    let _ret = read_line();
}
