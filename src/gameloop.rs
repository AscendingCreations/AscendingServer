mod handledata;
mod mainloop;

pub use handledata::{PacketRouter, SocketID, handle_data};
pub use mainloop::game_loop;
