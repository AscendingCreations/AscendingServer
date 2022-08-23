use crate::gametypes::MAX_NPCS;

#[derive(Debug, Clone, Copy, Derivative)]
#[derivative(Default(new = "true"))]
pub struct Achievements {
    #[derivative(Default(value = "[0; MAX_NPCS]"))]
    pub npckilled: [u32; MAX_NPCS],
    pub daykills: u32,
    pub nightkills: u32,
    pub survivekill: u32,
    pub revivals: u32,
    pub deaths: u32,
}
