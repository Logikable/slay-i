use rand::RngExt;

use crate::{
    actions::{
        create_card_in_discard::CreateCardInDiscardAction, damage::DamageAction,
        gain_status::GainStatusAction,
    },
    cards::CardClass,
    game::{CreatureRef, Rand},
    monster::{Intent, MonsterBehavior, MonsterInfo},
    move_history::MoveHistory,
    queue::ActionQueue,
    status::Status,
};

#[derive(PartialEq, Eq, Clone, Copy)]
enum Action {
    None,
    Slime,
    Frail,
}

#[derive(Clone)]
pub struct SlimeSpikeM {
    action: Action,
    history: MoveHistory<Action>,
}

impl SlimeSpikeM {
    pub fn new() -> Self {
        Self {
            action: Action::None,
            history: MoveHistory::new(),
        }
    }
}

impl MonsterBehavior for SlimeSpikeM {
    fn clone_box(&self) -> Box<dyn MonsterBehavior> {
        Box::new(self.clone())
    }
    fn name(&self) -> &'static str {
        "spike slime M"
    }
    fn hp_range(&self) -> (i32, i32) {
        (29, 34)
    }

    fn take_turn(&mut self, this: CreatureRef, queue: &mut ActionQueue, _: &MonsterInfo) {
        match self.action {
            Action::Slime => {
                queue.push_bot(DamageAction::from_monster(10, this));
                queue.push_bot(CreateCardInDiscardAction(CardClass::Slimed));
            }
            Action::Frail => {
                queue.push_bot(GainStatusAction {
                    status: Status::Frail,
                    amount: 1,
                    target: CreatureRef::player(),
                });
            }
            Action::None => unreachable!(),
        }
    }
    fn roll_next_action(&mut self, r: &mut Rand, _info: &MonsterInfo) {
        let next = if r.random_range(0..10) < 3 {
            if self.history.last_two(Action::Slime) {
                Action::Frail
            } else {
                Action::Slime
            }
        } else if self.history.last(Action::Frail) {
            Action::Slime
        } else {
            Action::Frail
        };
        self.history.add(next);
        self.action = next;
    }

    fn get_intent(&self) -> Intent {
        match self.action {
            Action::None => unreachable!(),
            Action::Slime => Intent::AttackDebuff(10, 1),
            Action::Frail => Intent::Debuff,
        }
    }
}
