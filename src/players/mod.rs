mod inv;
mod logic;
mod player;

pub use inv::*;
pub use logic::*;
pub use player::*;

pub const fn is_name_acceptable(n: char) -> bool {
    matches!(n, '!' | '$' | '&' | '_' | '~' | '0'..='9' | 'A'..='F' | 'a'..='f')
}

pub const fn is_password_acceptable(n: char) -> bool {
    matches!(n, '!' | '$' | '&' | '_' | '%' | '@' | '?' | '~' | '0'..='9' | 'A'..='F' | 'a'..='f')
}
