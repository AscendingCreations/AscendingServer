use crate::{gametypes::*, time_ext::MyInstant};

//shared data between player and npc
#[derive(Derivative, Debug, Clone, PartialEq, Eq)]
#[derivative(Default(new = "true"))]
pub struct Entity {
    pub etype: EntityType,
    pub killcount: u32,
    #[derivative(Default(value = "1"))]
    pub level: i32,
    pub pos: Position,
    #[derivative(Default(value = "[25, 2, 100]"))]
    pub vital: [i32; VITALS_MAX],
    #[derivative(Default(value = "[25, 2, 100]"))]
    pub vitalmax: [i32; VITALS_MAX],
    #[derivative(Default(value = "[0; VITALS_MAX]"))]
    pub vitalbuffs: [i32; VITALS_MAX],
    #[derivative(Default(value = "[0; VITALS_MAX]"))]
    pub regens: [u32; VITALS_MAX],
    #[derivative(Default(value = "[1; COMBAT_MAX]"))]
    pub cstat: Combatstats,
    pub buffs: BuffCombatstats,
    pub targettype: EntityType,
    pub targetpos: Position,
    #[derivative(Default(value = "MyInstant::now()"))]
    pub attacktimer: MyInstant,
    #[derivative(Default(value = "MyInstant::now()"))]
    pub killcounttimer: MyInstant,
    #[derivative(Default(value = "MyInstant::now()"))]
    pub deathtimer: MyInstant,
    #[derivative(Default(value = "MyInstant::now()"))]
    pub targettimer: MyInstant,
    #[derivative(Default(value = "MyInstant::now()"))]
    pub combattimer: MyInstant,
    #[derivative(Default(value = "MyInstant::now()"))]
    pub movetimer: MyInstant,
    #[derivative(Default(value = "MyInstant::now()"))]
    pub just_spawned: MyInstant,
    #[derivative(Default(value = "DeathType::Alive"))]
    pub life: DeathType,
    pub traps: Vec<usize>,
    pub dir: u8,
    pub hidden: bool,
    pub stunned: bool,
    pub incombat: bool,
    pub casting: bool,
    pub mode: NpcMode, //Player is always None
}

impl Entity {
    pub fn get_id(&self) -> usize {
        self.etype.get_id()
    }

    pub fn reset_target(&mut self) {
        self.targettype = EntityType::None;
        self.targetpos = Position::default();
    }

    pub fn add_up_vital(&self, vital: usize) -> i32 {
        let hp = self.vitalmax[vital].saturating_add(self.vitalbuffs[vital]);

        if hp.is_negative() || hp == 0 {
            1
        } else {
            hp
        }
    }
}
