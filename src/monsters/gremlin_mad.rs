use crate::{
    actions::damage::DamageAction,
    game::{CreatureRef, Rand},
    monster::{Intent, MonsterBehavior, MonsterInfo},
    queue::ActionQueue,
    status::Status,
};

#[derive(Clone)]
pub struct GremlinMad;

impl GremlinMad {
    pub fn new() -> Self {
        Self
    }
}

impl MonsterBehavior for GremlinMad {
    fn clone_box(&self) -> Box<dyn MonsterBehavior> {
        Box::new(self.clone())
    }
    fn name(&self) -> &'static str {
        "mad gremlin"
    }

    fn hp_range(&self) -> (i32, i32) {
        (21, 25)
    }

    fn pre_combat(&self, queue: &mut ActionQueue, this: CreatureRef, _rng: &mut Rand) {
        queue.push_bot(crate::actions::gain_status::GainStatusAction {
            status: Status::Angry,
            amount: 2,
            target: this,
        });
    }

    fn take_turn(&mut self, this: CreatureRef, queue: &mut ActionQueue, _: &MonsterInfo) {
        queue.push_bot(DamageAction::from_monster(5, this));
    }

    fn roll_next_action(&mut self, _r: &mut Rand, _info: &MonsterInfo) {}

    fn get_intent(&self) -> Intent {
        Intent::Attack(5, 1)
    }
}
