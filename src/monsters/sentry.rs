use crate::{
    actions::{create_card_in_discard::CreateCardInDiscardAction, damage::DamageAction},
    cards::CardClass,
    game::{CreatureRef, Rand},
    monster::{Intent, MonsterBehavior, MonsterInfo},
    queue::ActionQueue,
};

#[derive(PartialEq, Eq, Clone, Copy)]
enum Action {
    Attack,
    Debuff,
}

#[derive(Clone)]
pub struct Sentry {
    action: Action,
}

impl Sentry {
    pub fn new_debuff_first() -> Self {
        Self {
            action: Action::Attack,
        }
    }
    pub fn new_attack_first() -> Self {
        Self {
            action: Action::Debuff,
        }
    }
}

impl MonsterBehavior for Sentry {
    fn clone_box(&self) -> Box<dyn MonsterBehavior> {
        Box::new(self.clone())
    }
    fn name(&self) -> &'static str {
        "sentry"
    }
    fn hp_range(&self) -> (i32, i32) {
        (39, 45)
    }

    fn take_turn(&mut self, this: CreatureRef, queue: &mut ActionQueue, _: &MonsterInfo) {
        match self.action {
            Action::Attack => {
                queue.push_bot(DamageAction::from_monster(10, this));
            }
            Action::Debuff => {
                for _ in 0..2 {
                    queue.push_bot(CreateCardInDiscardAction(CardClass::Dazed));
                }
            }
        }
    }
    fn roll_next_action(&mut self, _: &mut Rand, _info: &MonsterInfo) {
        self.action = match self.action {
            Action::Attack => Action::Debuff,
            Action::Debuff => Action::Attack,
        };
    }

    fn get_intent(&self) -> Intent {
        match self.action {
            Action::Attack => Intent::Attack(10, 1),
            Action::Debuff => Intent::Debuff,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{assert_matches, combat::EndTurnStep, game::GameBuilder, relic::RelicClass};

    #[test]
    fn test_sentry_a() {
        let mut g = GameBuilder::default()
            .add_relic(RelicClass::RunicPyramid)
            .build_combat_with_monster(Sentry::new_debuff_first());
        for _ in 0..10 {
            g.add_card_to_hand(CardClass::Anger);
        }
        assert_matches!(g.monsters[0].behavior.get_intent(), Intent::Debuff);
        g.step_test(EndTurnStep);
        assert_eq!(g.discard_pile.len(), 2);
        assert_eq!(g.discard_pile[0].borrow().class, CardClass::Dazed);
        assert_eq!(g.discard_pile[1].borrow().class, CardClass::Dazed);

        assert_matches!(g.monsters[0].behavior.get_intent(), Intent::Attack(10, 1));
        g.step_test(EndTurnStep);
        assert_matches!(g.monsters[0].behavior.get_intent(), Intent::Debuff);
    }

    #[test]
    fn test_sentry_b() {
        let mut g = GameBuilder::default()
            .add_relic(RelicClass::RunicPyramid)
            .build_combat_with_monster(Sentry::new_attack_first());
        for _ in 0..10 {
            g.add_card_to_hand(CardClass::Anger);
        }
        g.step_test(EndTurnStep);
        assert_matches!(g.monsters[0].behavior.get_intent(), Intent::Debuff);
        g.step_test(EndTurnStep);
        assert_matches!(g.monsters[0].behavior.get_intent(), Intent::Attack(10, 1));
        g.step_test(EndTurnStep);
        assert_matches!(g.monsters[0].behavior.get_intent(), Intent::Debuff);
    }
}
