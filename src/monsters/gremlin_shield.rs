use crate::{
    actions::{block_random_monster::BlockRandomMonsterAction, damage::DamageAction},
    game::{CreatureRef, Rand},
    monster::{Intent, MonsterBehavior, MonsterInfo},
    queue::ActionQueue,
};

#[derive(Clone)]
pub struct GremlinShield {
    is_bashing: bool,
}

impl GremlinShield {
    pub fn new() -> Self {
        Self { is_bashing: false }
    }
}

impl MonsterBehavior for GremlinShield {
    fn clone_box(&self) -> Box<dyn MonsterBehavior> {
        Box::new(self.clone())
    }
    fn name(&self) -> &'static str {
        "shield gremlin"
    }

    fn hp_range(&self) -> (i32, i32) {
        (13, 17)
    }

    fn take_turn(&mut self, this: CreatureRef, queue: &mut ActionQueue, _: &MonsterInfo) {
        if self.is_bashing {
            queue.push_bot(DamageAction::from_monster(8, this));
        } else {
            queue.push_bot(BlockRandomMonsterAction {
                source: this,
                amount: 11,
            });
        }
    }

    fn roll_next_action(&mut self, _rng: &mut Rand, info: &MonsterInfo) {
        if !self.is_bashing && info.num_alive_monsters <= 1 {
            self.is_bashing = true;
        }
    }

    fn get_intent(&self) -> Intent {
        if self.is_bashing {
            Intent::Attack(8, 1)
        } else {
            Intent::Buff
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assert_matches;
    use crate::combat::EndTurnStep;
    use crate::game::GameBuilder;
    use crate::monsters::test::AttackMonster;
    use crate::status::Status;

    #[test]
    fn test_gremlin_shield_protect() {
        let mut g = GameBuilder::default()
            .build_combat_with_monsters(AttackMonster::new(1), GremlinShield::new());

        g.step_test(EndTurnStep);
        assert_eq!(g.monsters[0].creature.block, 11);

        g.player.set_status(Status::Thorns, 999);

        g.step_test(EndTurnStep);
        assert_eq!(g.monsters[1].creature.block, 11);

        assert_matches!(g.monsters[1].behavior.get_intent(), Intent::Attack(8, 1));
    }
}
