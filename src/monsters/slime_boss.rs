use crate::{
    actions::{
        create_card_in_discard::CreateCardInDiscardAction,
        damage::DamageAction,
        split_monster::{SplitMonsterAction, SplitMonsterType},
    },
    cards::CardClass,
    creature::Creature,
    game::{CreatureRef, Rand},
    monster::{Intent, MonsterBehavior, MonsterInfo},
    queue::ActionQueue,
};

#[derive(PartialEq, Eq, Clone, Copy)]
enum Action {
    GoopSpray,
    Preparing,
    Slam,
    Split,
}

#[derive(Clone)]
pub struct SlimeBoss {
    action: Action,
}

impl SlimeBoss {
    pub fn new() -> Self {
        Self {
            action: Action::GoopSpray,
        }
    }
}

impl MonsterBehavior for SlimeBoss {
    fn clone_box(&self) -> Box<dyn MonsterBehavior> {
        Box::new(self.clone())
    }
    fn name(&self) -> &'static str {
        "slime boss"
    }
    fn hp_range(&self) -> (i32, i32) {
        (150, 150)
    }

    fn on_take_damage(&mut self, _: CreatureRef, this_creature: &mut Creature) {
        if this_creature.cur_hp <= this_creature.max_hp / 2 {
            self.action = Action::Split;
        }
    }

    fn take_turn(&mut self, this: CreatureRef, queue: &mut ActionQueue, _: &MonsterInfo) {
        match self.action {
            Action::GoopSpray => {
                for _ in 0..5 {
                    queue.push_bot(CreateCardInDiscardAction(CardClass::Slimed));
                }
                self.action = Action::Preparing;
            }
            Action::Preparing => {
                self.action = Action::Slam;
            }
            Action::Slam => {
                queue.push_bot(DamageAction::from_monster(38, this));
                self.action = Action::GoopSpray;
            }
            Action::Split => {
                queue.push_bot(SplitMonsterAction {
                    monster: this,
                    ty: SplitMonsterType::SlimeBoss,
                });
            }
        }
    }
    fn roll_next_action(&mut self, _: &mut Rand, _: &MonsterInfo) {}

    fn get_intent(&self) -> Intent {
        match self.action {
            Action::GoopSpray => Intent::StrongDebuff,
            Action::Preparing => Intent::Unknown,
            Action::Slam => Intent::Attack(38, 1),
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
        let mut g = GameBuilder::default().build_combat_with_monster(SlimeBoss::new());
        let player_hp = g.player.cur_hp;
        assert_not_matches!(g.monsters[0].behavior.get_intent(), Intent::Unknown);
        g.run_action(DamageAction::thorns_no_rupture(74, CreatureRef::monster(0)));
        assert_not_matches!(g.monsters[0].behavior.get_intent(), Intent::Unknown);
        g.run_action(DamageAction::thorns_no_rupture(1, CreatureRef::monster(0)));
        assert_matches!(g.monsters[0].behavior.get_intent(), Intent::Unknown);
        g.step_test(EndTurnStep);
        assert_eq!(player_hp, g.player.cur_hp);
        assert_eq!(g.monsters.len(), 3);
        assert!(!g.monsters[0].creature.is_actionable());
        assert!(g.monsters[1].creature.is_actionable());
        assert!(g.monsters[2].creature.is_actionable());
        assert_eq!(g.monsters[1].creature.cur_hp, 75);
        assert_eq!(g.monsters[2].creature.cur_hp, 75);
        assert_eq!(
            g.get_actionable_monsters_in_order()
                .iter()
                .map(|&c| g.get_creature(c).name)
                .collect::<Vec<_>>(),
            vec!["spike slime L", "acid slime L"]
        );
    }
}
