use crate::{gametypes::*, time_ext::MyInstant};
use educe::Educe;

#[derive(Educe, Debug, Copy, Clone, PartialEq, Eq)]
#[educe(Default)]
pub struct Targeting {
    pub target_type: Target,
    pub target_pos: Position,
    #[educe(Default = MyInstant::now())]
    pub target_timer: MyInstant,
}
