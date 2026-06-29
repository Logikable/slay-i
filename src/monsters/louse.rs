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
    Start,
    BuffOrDebuff,
    Attack,
}

#[derive(Clone)]
pub struct Louse {
    action: Action,
    damage: i32,
    history: MoveHistory<Action>,
    is_red: bool,
}

impl Louse {
    pub fn red(rng: &mut Rand) -> Self {
        Self {
            action: Action::Start,
            damage: rng.random_range(6..=8),
            history: MoveHistory::new(),
            is_red: true,
        }
    }
    pub fn green(rng: &mut Rand) -> Self {
        Self {
            action: Action::Start,
            damage: rng.random_range(6..=8),
            history: MoveHistory::new(),
            is_red: false,
        }
    }
}

impl MonsterBehavior for Louse {
    fn clone_box(&self) -> Box<dyn MonsterBehavior> {
        Box::new(self.clone())
    }
    fn name(&self) -> &'static str {
        if self.is_red {
            "red louse"
        } else {
            "green louse"
        }
    }

    fn hp_range(&self) -> (i32, i32) {
        if self.is_red { (11, 16) } else { (12, 18) }
    }

    fn pre_combat(&self, queue: &mut ActionQueue, this: CreatureRef, rng: &mut Rand) {
        queue.push_bot(GainStatusAction {
            status: Status::CurlUp,
            amount: rng.random_range(9..=12),
            target: this,
        });
    }

    fn take_turn(&mut self, this: CreatureRef, queue: &mut ActionQueue, _: &MonsterInfo) {
        match self.action {
            Action::Start => panic!(),
            Action::BuffOrDebuff => queue.push_bot(if self.is_red {
                GainStatusAction {
                    status: Status::Strength,
                    amount: 5,
                    target: this,
                }
            } else {
                GainStatusAction {
                    status: Status::Weak,
                    amount: 2,
                    target: CreatureRef::player(),
                }
            }),
            Action::Attack => queue.push_bot(DamageAction::from_monster(self.damage, this)),
        }
    }
    fn roll_next_action(&mut self, rng: &mut Rand, _info: &MonsterInfo) {
        let next = match rng.random_range(0..4) {
            0 => {
                if self.history.last(Action::BuffOrDebuff) {
                    Action::Attack
                } else {
                    Action::BuffOrDebuff
                }
            }
            _ => {
                if self.history.last_two(Action::Attack) {
                    Action::BuffOrDebuff
                } else {
                    Action::Attack
                }
            }
        };
        self.history.add(next);
        self.action = next;
    }

    fn get_intent(&self) -> Intent {
        match self.action {
            Action::Start => panic!(),
            Action::BuffOrDebuff => {
                if self.is_red {
                    Intent::Buff
                } else {
                    Intent::Debuff
                }
            }
            Action::Attack => Intent::Attack(self.damage, 1),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{combat::EndTurnStep, game::GameBuilder};

    #[test]
    fn test_red_louse() {
        for _ in 0..10 {
            let mut g = GameBuilder::default().build_combat_with_monster_rng(Louse::red);
            assert!(g.monsters[0].creature.has_status(Status::CurlUp));
            let mut num_consecutive_attacks = 0;
            let mut num_consecutive_buffs = 0;
            g.player.max_hp = 100;
            for _ in 0..10 {
                g.player.cur_hp = 100;
                match g.monsters[0].behavior.get_intent() {
                    Intent::Attack(..) => {
                        num_consecutive_attacks += 1;
                        num_consecutive_buffs = 0;
                    }
                    Intent::Buff => {
                        num_consecutive_buffs += 1;
                        num_consecutive_attacks = 0;
                    }
                    _ => panic!(),
                }
                assert!(num_consecutive_attacks <= 2);
                assert!(num_consecutive_buffs <= 2);
                g.step_test(EndTurnStep);
            }
            assert!(g.monsters[0].creature.has_status(Status::Strength));
        }
    }
}
