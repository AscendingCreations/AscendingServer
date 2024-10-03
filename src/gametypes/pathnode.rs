use super::*;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct PathNode {
    pub g: i32,
    pub h: i32,
    pub f: i32,
    pub parent: Option<usize>,
    pub pos: Position,
    pub dir: Dir,
    pub offset: Position,
}

impl PathNode {
    pub fn new(pos: Position, dir: Dir, offset: Position, parent: Option<usize>) -> Self {
        Self {
            g: 0,
            h: 0,
            f: 0,
            parent,
            pos,
            dir,
            offset,
        }
    }
}
