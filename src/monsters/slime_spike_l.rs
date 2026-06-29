use rand::RngExt;

use crate::{
    actions::{
        create_card_in_discard::CreateCardInDiscardAction,
        damage::DamageAction,
        gain_status::GainStatusAction,
        split_monster::{SplitMonsterAction, SplitMonsterType},
    },
    cards::CardClass,
    creature::Creature,
    game::{CreatureRef, Rand},
    monster::{Intent, MonsterBehavior, MonsterInfo},
    move_history::MoveHistory,
    queue::ActionQueue,
    status::Status,
};

#[derive(PartialEq, Eq, Clone, Copy)]
enum Action {
    Start,
    Lick,
    Tackle,
    Split,
}

#[derive(Clone)]
pub struct SlimeSpikeL {
    action: Action,
    history: MoveHistory<Action>,
}

impl SlimeSpikeL {
    pub fn new() -> Self {
        Self {
            action: Action::Start,
            history: MoveHistory::new(),
        }
    }
}

impl MonsterBehavior for SlimeSpikeL {
    fn clone_box(&self) -> Box<dyn MonsterBehavior> {
        Box::new(self.clone())
    }
    fn name(&self) -> &'static str {
        "spike slime L"
    }
    fn hp_range(&self) -> (i32, i32) {
        (67, 73)
    }

    fn on_take_damage(&mut self, _: CreatureRef, this_creature: &mut Creature) {
        if this_creature.cur_hp <= this_creature.max_hp / 2 {
            self.action = Action::Split;
        }
    }

    fn take_turn(&mut self, this: CreatureRef, queue: &mut ActionQueue, _: &MonsterInfo) {
        match self.action {
            Action::Start => panic!(),
            Action::Lick => queue.push_bot(GainStatusAction {
                status: Status::Frail,
                amount: 3,
                target: CreatureRef::player(),
            }),
            Action::Tackle => {
                queue.push_bot(DamageAction::from_monster(18, this));

                for _ in 0..2 {
                    queue.push_bot(CreateCardInDiscardAction(CardClass::Slimed));
                }
            }
            Action::Split => {
                queue.push_bot(SplitMonsterAction {
                    monster: this,
                    ty: SplitMonsterType::SlimeSpikeL,
                });
            }
        }
    }
    fn roll_next_action(&mut self, rng: &mut Rand, _info: &MonsterInfo) {
        let next = if rng.random_range(0..10) < 3 {
            if self.history.last_two(Action::Tackle) {
                Action::Lick
            } else {
                Action::Tackle
            }
        } else if self.history.last(Action::Lick) {
            Action::Tackle
        } else {
            Action::Lick
        };
        self.action = next;
        self.history.add(next);
    }

    fn get_intent(&self) -> Intent {
        match self.action {
            Action::Start => panic!(),
            Action::Lick => Intent::Debuff,
            Action::Tackle => Intent::AttackDebuff(18, 1),
            Action::Split => Intent::Unknown,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{assert_matches, assert_not_matches, combat::EndTurnStep, game::GameBuilder};

    #[test]
    fn test_split() {
        let mut g = GameBuilder::default().build_combat_with_monster(SlimeSpikeL::new());
        let player_hp = g.player.cur_hp;
        g.monsters[0].creature.max_hp = 50;
        g.monsters[0].creature.cur_hp = 50;
        assert_not_matches!(g.monsters[0].behavior.get_intent(), Intent::Unknown);
        g.run_action(DamageAction::thorns_no_rupture(24, CreatureRef::monster(0)));
        assert_not_matches!(g.monsters[0].behavior.get_intent(), Intent::Unknown);
        g.run_action(DamageAction::thorns_no_rupture(1, CreatureRef::monster(0)));
        assert_matches!(g.monsters[0].behavior.get_intent(), Intent::Unknown);
        g.step_test(EndTurnStep);
        assert_eq!(player_hp, g.player.cur_hp);
        assert_eq!(g.monsters.len(), 3);
        assert!(!g.monsters[0].creature.is_actionable());
        assert!(g.monsters[1].creature.is_actionable());
        assert!(g.monsters[2].creature.is_actionable());
        assert_eq!(g.monsters[1].creature.cur_hp, 25);
        assert_eq!(g.monsters[2].creature.cur_hp, 25);
        assert_eq!(g.monsters[1].creature.name, "spike slime M");
        assert_eq!(g.monsters[2].creature.name, "spike slime M");
    }
}
