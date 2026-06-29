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
    Entangle,
    Scrape,
}

#[derive(Clone)]
pub struct RedSlaver {
    action: Action,
    history: MoveHistory<Action>,
    used_entangle: bool,
}

impl RedSlaver {
    pub fn new() -> Self {
        Self {
            action: Action::None,
            history: MoveHistory::new(),
            used_entangle: false,
        }
    }
}

impl MonsterBehavior for RedSlaver {
    fn clone_box(&self) -> Box<dyn MonsterBehavior> {
        Box::new(self.clone())
    }
    fn name(&self) -> &'static str {
        "red slaver"
    }

    fn hp_range(&self) -> (i32, i32) {
        (48, 52)
    }

    fn take_turn(&mut self, this: CreatureRef, queue: &mut ActionQueue, _: &MonsterInfo) {
        match self.action {
            Action::Stab => {
                queue.push_bot(DamageAction::from_monster(14, this));
            }
            Action::Entangle => {
                queue.push_bot(GainStatusAction {
                    status: Status::Entangled,
                    amount: 1,
                    target: CreatureRef::player(),
                });
                self.used_entangle = true;
            }
            Action::Scrape => {
                queue.push_bot(DamageAction::from_monster(9, this));
                queue.push_bot(GainStatusAction {
                    status: Status::Vulnerable,
                    amount: 2,
                    target: CreatureRef::player(),
                });
            }
            Action::None => unreachable!(),
        }
    }

    fn roll_next_action(&mut self, r: &mut Rand, _info: &MonsterInfo) {
        let next = if self.action == Action::None {
            Action::Stab
        } else {
            let num = r.random_range(0..100);

            if num >= 75 && !self.used_entangle {
                Action::Entangle
            } else if num >= 55 && self.used_entangle && !self.history.last_two(Action::Stab) {
                Action::Stab
            } else if !self.history.last(Action::Scrape) {
                Action::Scrape
            } else {
                Action::Stab
            }
        };

        self.history.add(next);
        self.action = next;
    }

    fn get_intent(&self) -> Intent {
        match self.action {
            Action::None => unreachable!(),
            Action::Stab => Intent::Attack(14, 1),
            Action::Entangle => Intent::StrongDebuff,
            Action::Scrape => Intent::AttackDebuff(9, 1),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{combat::EndTurnStep, game::GameBuilder};

    #[test]
    fn test_red_slaver() {
        let mut g = GameBuilder::default().build_combat_with_monster(RedSlaver::new());
        let mut has_entanged = false;

        for _ in 0..10 {
            g.player.cur_hp = 50;
            g.step_test(EndTurnStep);
            if matches!(g.monsters[0].behavior.get_intent(), Intent::StrongDebuff) {
                assert!(!has_entanged);
                has_entanged = true;
            }
        }
    }
}
