use crate::{
    actions::{damage::DamageAction, gain_status::GainStatusAction},
    game::{CreatureRef, Rand},
    monster::{Intent, MonsterBehavior, MonsterInfo},
    queue::ActionQueue,
    status::Status,
};

#[derive(PartialEq, Eq, Clone, Copy)]
enum Action {
    None,
    Attack,
    Weaken,
}

#[derive(Clone)]
pub struct SlimeAcidS {
    action: Action,
}

impl SlimeAcidS {
    pub fn new() -> Self {
        Self {
            action: Action::None,
        }
    }
}

impl MonsterBehavior for SlimeAcidS {
    fn clone_box(&self) -> Box<dyn MonsterBehavior> {
        Box::new(self.clone())
    }
    fn name(&self) -> &'static str {
        "spike slime S"
    }
    fn hp_range(&self) -> (i32, i32) {
        (9, 13)
    }

    fn take_turn(&mut self, this: CreatureRef, queue: &mut ActionQueue, _: &MonsterInfo) {
        match self.action {
            Action::Attack => {
                queue.push_bot(DamageAction::from_monster(4, this));
            }
            Action::Weaken => {
                queue.push_bot(GainStatusAction {
                    status: Status::Weak,
                    amount: 1,
                    target: CreatureRef::player(),
                });
            }
            Action::None => unreachable!(),
        }
    }
    fn roll_next_action(&mut self, _: &mut Rand, _info: &MonsterInfo) {
        self.action = match self.action {
            Action::None | Action::Attack => Action::Weaken,
            Action::Weaken => Action::Attack,
        };
    }

    fn get_intent(&self) -> Intent {
        match self.action {
            Action::None => unreachable!(),
            Action::Attack => Intent::Attack(4, 1),
            Action::Weaken => Intent::Debuff,
        }
    }
}
