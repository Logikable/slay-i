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
    Attack,
    Slime,
    Weaken,
}

#[derive(Clone)]
pub struct SlimeAcidM {
    action: Action,
    history: MoveHistory<Action>,
}

impl SlimeAcidM {
    pub fn new() -> Self {
        Self {
            action: Action::None,
            history: MoveHistory::new(),
        }
    }
}

impl MonsterBehavior for SlimeAcidM {
    fn clone_box(&self) -> Box<dyn MonsterBehavior> {
        Box::new(self.clone())
    }
    fn name(&self) -> &'static str {
        "acid slime M"
    }
    fn hp_range(&self) -> (i32, i32) {
        (29, 34)
    }

    fn take_turn(&mut self, this: CreatureRef, queue: &mut ActionQueue, _: &MonsterInfo) {
        match self.action {
            Action::Attack => {
                queue.push_bot(DamageAction::from_monster(12, this));
            }
            Action::Slime => {
                queue.push_bot(DamageAction::from_monster(8, this));
                queue.push_bot(CreateCardInDiscardAction(CardClass::Slimed));
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
    fn roll_next_action(&mut self, r: &mut Rand, _info: &MonsterInfo) {
        let next = match r.random_range(0..10) {
            0..4 => {
                if self.history.last_two(Action::Slime) {
                    if r.random() {
                        Action::Attack
                    } else {
                        Action::Weaken
                    }
                } else {
                    Action::Slime
                }
            }
            4..8 => {
                if self.history.last_two(Action::Attack) {
                    if r.random() {
                        Action::Slime
                    } else {
                        Action::Weaken
                    }
                } else {
                    Action::Attack
                }
            }
            _ => {
                if self.history.last(Action::Weaken) {
                    if r.random_range(0..10) < 4 {
                        Action::Slime
                    } else {
                        Action::Attack
                    }
                } else {
                    Action::Weaken
                }
            }
        };
        self.history.add(next);
        self.action = next;
    }

    fn get_intent(&self) -> Intent {
        match self.action {
            Action::None => unreachable!(),
            Action::Attack => Intent::Attack(12, 1),
            Action::Slime => Intent::AttackDebuff(8, 1),
            Action::Weaken => Intent::Debuff,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{cards::CardClass, combat::EndTurnStep, game::GameBuilder, relic::RelicClass};

    #[test]
    fn test_add_slimed() {
        let mut g = GameBuilder::default()
            .add_relic(RelicClass::RunicPyramid)
            .add_cards(CardClass::Strike, 10)
            .build_combat_with_monster(SlimeAcidM::new());
        let mut found_slimed = false;
        for _ in 0..50 {
            g.player.cur_hp = 50;
            g.step_test(EndTurnStep);
            if !g.discard_pile.is_empty() && g.discard_pile[0].borrow().class == CardClass::Slimed {
                found_slimed = true;
                break;
            }
        }
        assert!(found_slimed);
    }
}
