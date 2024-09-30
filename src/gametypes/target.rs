use crate::{gametypes::*, time_ext::MyInstant};
use educe::Educe;

#[derive(Educe, Debug, Copy, Clone, PartialEq, Eq)]
#[educe(Default)]
pub struct Targeting {
    pub target: Target,
    #[educe(Default = MyInstant::now())]
    pub timer: MyInstant,
}

impl Targeting {
    pub fn update_pos(&mut self, position: Position) {
        self.target.update_pos(position);
    }

    pub fn get_pos(&self) -> Option<Position> {
        self.target.get_pos()
    }
}
