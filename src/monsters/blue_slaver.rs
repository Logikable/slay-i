use rand::RngExt;

use crate::{
    actions::{damage::DamageAction, gain_status::GainStatusAction},
    game::{CreatureRef, Rand},
    monster::{Intent, MonsterBehavior, MonsterInfo},
    move_history::MoveHistory,
    queue::ActionQueue,
    status::Status,
};

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
enum Action {
    None,
    Stab,
    Rake,
}

#[derive(Clone)]
pub struct BlueSlaver {
    action: Action,
    history: MoveHistory<Action>,
}

impl BlueSlaver {
    pub fn new() -> Self {
        Self {
            action: Action::None,
            history: MoveHistory::new(),
        }
    }
}

impl MonsterBehavior for BlueSlaver {
    fn clone_box(&self) -> Box<dyn MonsterBehavior> {
        Box::new(self.clone())
    }
    fn name(&self) -> &'static str {
        "blue slaver"
    }

    fn hp_range(&self) -> (i32, i32) {
        (48, 52)
    }

    fn take_turn(&mut self, this: CreatureRef, queue: &mut ActionQueue, _: &MonsterInfo) {
        match self.action {
            Action::Stab => {
                queue.push_bot(DamageAction::from_monster(13, this));
            }
            Action::Rake => {
                queue.push_bot(DamageAction::from_monster(8, this));
                queue.push_bot(GainStatusAction {
                    status: Status::Weak,
                    amount: 2,
                    target: CreatureRef::player(),
                });
            }
            Action::None => unreachable!(),
        }
    }

    fn roll_next_action(&mut self, r: &mut Rand, _info: &MonsterInfo) {
        let next = if r.random_range(0..10) >= 4 && !self.history.last_two(Action::Stab) {
            Action::Stab
        } else if !self.history.last(Action::Rake) {
            Action::Rake
        } else {
            Action::Stab
        };

        self.history.add(next);
        self.action = next;
    }

    fn get_intent(&self) -> Intent {
        match self.action {
            Action::None => unreachable!(),
            Action::Stab => Intent::Attack(13, 1),
            Action::Rake => Intent::AttackDebuff(8, 1),
        }
    }
}
