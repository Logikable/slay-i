use crate::{
    actions::damage::DamageAction,
    game::{CreatureRef, Rand},
    monster::{Intent, MonsterBehavior, MonsterInfo},
    queue::ActionQueue,
};

#[derive(Clone)]
pub struct GremlinSneaky;

impl GremlinSneaky {
    pub fn new() -> Self {
        Self
    }
}

impl MonsterBehavior for GremlinSneaky {
    fn clone_box(&self) -> Box<dyn MonsterBehavior> {
        Box::new(self.clone())
    }
    fn name(&self) -> &'static str {
        "sneaky gremlin"
    }

    fn hp_range(&self) -> (i32, i32) {
        (11, 15)
    }

    fn take_turn(&mut self, this: CreatureRef, queue: &mut ActionQueue, _: &MonsterInfo) {
        queue.push_bot(DamageAction::from_monster(10, this));
    }

    fn roll_next_action(&mut self, _rng: &mut Rand, _info: &MonsterInfo) {}

    fn get_intent(&self) -> Intent {
        Intent::Attack(10, 1)
    }
}
