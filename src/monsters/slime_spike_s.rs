use crate::{
    actions::damage::DamageAction,
    game::{CreatureRef, Rand},
    monster::{Intent, MonsterBehavior, MonsterInfo},
    queue::ActionQueue,
};

#[derive(Clone)]
pub struct SlimeSpikeS;

impl SlimeSpikeS {
    pub fn new() -> Self {
        Self
    }
}

impl MonsterBehavior for SlimeSpikeS {
    fn clone_box(&self) -> Box<dyn MonsterBehavior> {
        Box::new(self.clone())
    }
    fn name(&self) -> &'static str {
        "spike slime S"
    }
    fn hp_range(&self) -> (i32, i32) {
        (11, 15)
    }

    fn take_turn(&mut self, this: CreatureRef, queue: &mut ActionQueue, _: &MonsterInfo) {
        queue.push_bot(DamageAction::from_monster(6, this));
    }
    fn roll_next_action(&mut self, _: &mut Rand, _info: &MonsterInfo) {}

    fn get_intent(&self) -> Intent {
        Intent::Attack(6, 1)
    }
}
