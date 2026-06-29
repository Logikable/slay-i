use crate::{
    actions::{damage::DamageAction, gain_status::GainStatusAction},
    game::{CreatureRef, Rand},
    monster::{Intent, MonsterBehavior, MonsterInfo},
    queue::ActionQueue,
    status::Status,
};

#[derive(Clone)]
pub struct GremlinFat;

impl GremlinFat {
    pub fn new() -> Self {
        Self
    }
}

impl MonsterBehavior for GremlinFat {
    fn clone_box(&self) -> Box<dyn MonsterBehavior> {
        Box::new(self.clone())
    }
    fn name(&self) -> &'static str {
        "fat gremlin"
    }

    fn hp_range(&self) -> (i32, i32) {
        (14, 18)
    }

    fn take_turn(&mut self, this: CreatureRef, queue: &mut ActionQueue, _: &MonsterInfo) {
        queue.push_bot(DamageAction::from_monster(5, this));
        queue.push_bot(GainStatusAction {
            status: Status::Weak,
            amount: 1,
            target: CreatureRef::player(),
        });
        queue.push_bot(GainStatusAction {
            status: Status::Frail,
            amount: 1,
            target: CreatureRef::player(),
        });
    }

    fn roll_next_action(&mut self, _r: &mut Rand, _info: &MonsterInfo) {}

    fn get_intent(&self) -> Intent {
        Intent::AttackDebuff(5, 1)
    }
}
