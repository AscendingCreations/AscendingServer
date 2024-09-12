//mod combat;
//mod inv;
//mod logic;
//pub mod movement;
mod player;
//mod player_storage;

//pub use combat::*;
//pub use inv::*;
//pub use logic::*;
//pub use movement::*;
pub use player::*;
//pub use player_storage::*;

pub const fn is_name_acceptable(n: char) -> bool {
    matches!(n, '!' | '$' | '&' | '_' | '~' | '0'..='9' | 'A'..='Z' | 'a'..='z')
}

pub const fn is_password_acceptable(n: char) -> bool {
    matches!(n, '!' | '$' | '&' | '_' | '%' | '@' | '?' | '~' | '0'..='9' | 'A'..='Z' | 'a'..='z')
}
