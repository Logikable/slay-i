use rand::RngExt;

use crate::{
    actions::{damage::DamageAction, gain_status::GainStatusAction},
    game::{CreatureRef, Rand},
    monster::{Intent, MonsterBehavior, MonsterInfo},
    move_history::MoveHistory,
    queue::ActionQueue,
    status::Status,
};

#[derive(PartialEq, Eq, Clone, Copy)]
enum Action {
    None,
    Attack,
    Buff,
}

#[derive(Clone)]
pub struct FungiBeast {
    action: Action,
    history: MoveHistory<Action>,
}

impl FungiBeast {
    pub fn new() -> Self {
        Self {
            action: Action::None,
            history: MoveHistory::new(),
        }
    }
}

impl MonsterBehavior for FungiBeast {
    fn clone_box(&self) -> Box<dyn MonsterBehavior> {
        Box::new(self.clone())
    }
    fn name(&self) -> &'static str {
        "fungi beast"
    }
    fn hp_range(&self) -> (i32, i32) {
        (24, 28)
    }

    fn pre_combat(&self, queue: &mut ActionQueue, this: CreatureRef, _: &mut Rand) {
        queue.push_bot(GainStatusAction {
            status: Status::SporeCloud,
            amount: 2,
            target: this,
        });
    }
    fn take_turn(&mut self, this: CreatureRef, queue: &mut ActionQueue, _: &MonsterInfo) {
        match self.action {
            Action::Attack => {
                queue.push_bot(DamageAction::from_monster(6, this));
            }
            Action::Buff => {
                queue.push_bot(GainStatusAction {
                    status: Status::Strength,
                    amount: 5,
                    target: this,
                });
            }
            Action::None => unreachable!(),
        }
    }
    fn roll_next_action(&mut self, r: &mut Rand, _info: &MonsterInfo) {
        let next = if r.random_range(0..10) < 6 {
            if self.history.last_two(Action::Attack) {
                Action::Buff
            } else {
                Action::Attack
            }
        } else if self.history.last(Action::Buff) {
            Action::Attack
        } else {
            Action::Buff
        };
        self.history.add(next);
        self.action = next;
    }

    fn get_intent(&self) -> Intent {
        match self.action {
            Action::None => unreachable!(),
            Action::Attack => Intent::Attack(6, 1),
            Action::Buff => Intent::Buff,
        }
    }
}
