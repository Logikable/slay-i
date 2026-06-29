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
    Bellow,
    SkullBash,
    Rush,
}

#[derive(Clone)]
pub struct GremlinNob {
    action: Action,
    history: MoveHistory<Action>,
}

impl GremlinNob {
    pub fn new() -> Self {
        Self {
            action: Action::None,
            history: MoveHistory::new(),
        }
    }
}

impl MonsterBehavior for GremlinNob {
    fn clone_box(&self) -> Box<dyn MonsterBehavior> {
        Box::new(self.clone())
    }
    fn name(&self) -> &'static str {
        "gremlin nob"
    }

    fn hp_range(&self) -> (i32, i32) {
        (85, 90)
    }

    fn take_turn(&mut self, this: CreatureRef, queue: &mut ActionQueue, _: &MonsterInfo) {
        match self.action {
            Action::None => unreachable!(),
            Action::Bellow => {
                queue.push_bot(GainStatusAction {
                    status: Status::Enrage,
                    amount: 3,
                    target: this,
                });
            }
            Action::SkullBash => {
                queue.push_bot(DamageAction::from_monster(8, this));
                queue.push_bot(GainStatusAction {
                    status: Status::Vulnerable,
                    amount: 2,
                    target: CreatureRef::player(),
                });
            }
            Action::Rush => {
                queue.push_bot(DamageAction::from_monster(16, this));
            }
        }
    }

    fn roll_next_action(&mut self, _rng: &mut Rand, _info: &MonsterInfo) {
        let next = if self.action == Action::None {
            Action::Bellow
        } else if !self.history.last(Action::SkullBash)
            && !self.history.last_last(Action::SkullBash)
        {
            Action::SkullBash
        } else {
            Action::Rush
        };

        self.history.add(next);
        self.action = next;
    }

    fn get_intent(&self) -> Intent {
        match self.action {
            Action::None => unreachable!(),
            Action::Bellow => Intent::Buff,
            Action::SkullBash => Intent::AttackDebuff(8, 1),
            Action::Rush => Intent::Attack(16, 1),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cards::CardClass;
    use crate::combat::EndTurnStep;
    use crate::game::GameBuilder;

    #[test]
    fn test_gremlin_nob_logic() {
        let mut g = GameBuilder::default().build_combat_with_monster(GremlinNob::new());
        g.play_card(CardClass::SeeingRed, None);
        g.step_test(EndTurnStep);

        assert_eq!(g.monsters[0].creature.get_status(Status::Strength), None);
        g.play_card(CardClass::SeeingRed, None);
        assert_eq!(g.monsters[0].creature.get_status(Status::Strength), Some(3));
        g.monsters[0].creature.clear_all_status();

        g.player.cur_hp = 50;
        g.step_test(EndTurnStep);
        assert_eq!(g.player.cur_hp, 50 - 8);

        g.player.cur_hp = 50;
        g.step_test(EndTurnStep);
        assert_eq!(g.player.cur_hp, 50 - 24);

        g.player.cur_hp = 50;
        g.step_test(EndTurnStep);
        assert_eq!(g.player.cur_hp, 50 - 24);

        g.player.cur_hp = 50;
        g.step_test(EndTurnStep);
        assert_eq!(g.player.cur_hp, 50 - 8);
    }
}
