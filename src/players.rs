mod inv;
mod logic;
mod player;
mod combat;
pub mod movement;

pub use combat::*;
pub use inv::*;
pub use logic::*;
pub use player::*;
pub use movement::*;

pub const fn is_name_acceptable(n: char) -> bool {
    matches!(n, '!' | '$' | '&' | '_' | '~' | '0'..='9' | 'A'..='Z' | 'a'..='z')
}

pub const fn is_password_acceptable(n: char) -> bool {
    matches!(n, '!' | '$' | '&' | '_' | '%' | '@' | '?' | '~' | '0'..='9' | 'A'..='Z' | 'a'..='z')
}
