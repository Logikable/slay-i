use rand::RngExt;

use crate::{
    actions::{
        block::BlockAction, damage::DamageAction, escape_monster::EscapeMonsterAction,
        rob::RobAction,
    },
    game::{CreatureRef, Rand},
    monster::{Intent, MonsterBehavior, MonsterInfo},
    queue::ActionQueue,
};

#[derive(PartialEq, Eq, Clone, Copy)]
enum Action {
    Mug,
    Lunge,
    SmokeBomb,
    Escape,
}

#[derive(Clone)]
pub struct Looter {
    action: Action,
    turn: i32,
}

impl Looter {
    pub fn new() -> Self {
        Self {
            action: Action::Mug,
            turn: 0,
        }
    }
}

impl MonsterBehavior for Looter {
    fn clone_box(&self) -> Box<dyn MonsterBehavior> {
        Box::new(self.clone())
    }
    fn name(&self) -> &'static str {
        "looter"
    }
    fn hp_range(&self) -> (i32, i32) {
        (46, 50)
    }

    fn take_turn(&mut self, this: CreatureRef, queue: &mut ActionQueue, _: &MonsterInfo) {
        match self.action {
            Action::Mug => {
                queue.push_bot(DamageAction::from_monster(11, this));
                queue.push_bot(RobAction {
                    source: this,
                    amount: 20,
                });
            }
            Action::Lunge => {
                queue.push_bot(DamageAction::from_monster(14, this));
                queue.push_bot(RobAction {
                    source: this,
                    amount: 20,
                });
            }
            Action::SmokeBomb => queue.push_bot(BlockAction::monster(this, 6)),
            Action::Escape => queue.push_bot(EscapeMonsterAction(this)),
        }
        self.turn += 1;
    }
    fn roll_next_action(&mut self, rng: &mut Rand, _info: &MonsterInfo) {
        let next = match self.action {
            Action::Mug => {
                if self.turn == 2 {
                    if rng.random() {
                        Action::Lunge
                    } else {
                        Action::SmokeBomb
                    }
                } else {
                    Action::Mug
                }
            }
            Action::Lunge => Action::SmokeBomb,
            Action::SmokeBomb => Action::Escape,
            Action::Escape => Action::Escape,
        };
        self.action = next;
    }

    fn get_intent(&self) -> Intent {
        match self.action {
            Action::Mug => Intent::Attack(11, 1),
            Action::Lunge => Intent::Attack(14, 1),
            Action::SmokeBomb => Intent::Defend,
            Action::Escape => Intent::Escape,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{combat::EndTurnStep, game::GameBuilder, status::Status};

    #[test]
    fn test_basic() {
        let mut g = GameBuilder::default().build_combat_with_monster(Looter::new());
        g.gold = 25;
        g.step_test(EndTurnStep);
        assert_eq!(g.gold, 5);
        assert_eq!(
            g.monsters[0].creature.get_status(Status::StolenGold),
            Some(20)
        );
    }

    #[test]
    fn test_kill_rewards() {}
}
