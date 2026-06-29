use crate::{
    actions::{
        block::BlockAction, create_card_in_discard::CreateCardInDiscardAction,
        damage::DamageAction, gain_status::GainStatusAction, upgrade_burns::UpgradeBurnsAction,
    },
    cards::CardClass,
    game::{CreatureRef, Rand},
    monster::{Intent, MonsterBehavior, MonsterInfo},
    queue::ActionQueue,
    status::Status,
};

#[derive(PartialEq, Eq, Clone, Copy)]
enum Action {
    Activate,
    Divider,
    Sear1,
    Sear2,
    Sear3,
    Tackle1,
    Tackle2,
    Inflame,
    Inferno,
}

#[derive(Clone)]
pub struct Hexaghost {
    action: Action,
    divider_amount: i32,
    upgraded_burns: bool,
}

impl Hexaghost {
    pub fn new() -> Self {
        Self {
            action: Action::Activate,
            divider_amount: 0,
            upgraded_burns: false,
        }
    }

    fn burn_class(&self) -> CardClass {
        if self.upgraded_burns {
            CardClass::BurnPlus
        } else {
            CardClass::Burn
        }
    }
}

impl MonsterBehavior for Hexaghost {
    fn clone_box(&self) -> Box<dyn MonsterBehavior> {
        Box::new(self.clone())
    }
    fn name(&self) -> &'static str {
        "hexaghost"
    }
    fn hp_range(&self) -> (i32, i32) {
        (264, 264)
    }

    fn take_turn(&mut self, this: CreatureRef, queue: &mut ActionQueue, info: &MonsterInfo) {
        match self.action {
            Action::Activate => {
                self.divider_amount = info.player_hp / 12 + 1;
            }
            Action::Divider => {
                for _ in 0..6 {
                    queue.push_bot(DamageAction::from_monster(self.divider_amount, this));
                }
            }
            Action::Sear1 | Action::Sear2 | Action::Sear3 => {
                queue.push_bot(DamageAction::from_monster(6, this));
                for _ in 0..2 {
                    queue.push_bot(CreateCardInDiscardAction(self.burn_class()));
                }
            }
            Action::Tackle1 | Action::Tackle2 => {
                for _ in 0..2 {
                    queue.push_bot(DamageAction::from_monster(6, this));
                }
            }
            Action::Inflame => {
                queue.push_bot(BlockAction::monster(this, 12));
                queue.push_bot(GainStatusAction {
                    status: Status::Strength,
                    amount: 3,
                    target: this,
                });
            }
            Action::Inferno => {
                self.upgraded_burns = true;
                for _ in 0..6 {
                    queue.push_bot(DamageAction::from_monster(3, this));
                }
                queue.push_bot(UpgradeBurnsAction());
                for _ in 0..3 {
                    queue.push_bot(CreateCardInDiscardAction(self.burn_class()));
                }
            }
        }

        self.action = match self.action {
            Action::Activate => Action::Divider,
            Action::Divider => Action::Sear1,
            Action::Sear1 => Action::Tackle1,
            Action::Sear2 => Action::Inflame,
            Action::Sear3 => Action::Inferno,
            Action::Tackle1 => Action::Sear2,
            Action::Tackle2 => Action::Sear3,
            Action::Inflame => Action::Tackle2,
            Action::Inferno => Action::Sear1,
        };
    }

    fn roll_next_action(&mut self, _: &mut Rand, _info: &MonsterInfo) {}

    fn get_intent(&self) -> Intent {
        match self.action {
            Action::Activate => Intent::Unknown,
            Action::Divider => Intent::Attack(self.divider_amount, 6),
            Action::Sear1 | Action::Sear2 | Action::Sear3 => Intent::AttackDebuff(6, 1),
            Action::Tackle1 | Action::Tackle2 => Intent::Attack(2, 6),
            Action::Inflame => Intent::DefendBuff,
            Action::Inferno => Intent::Attack(3, 6),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        actions::increase_max_hp::IncreaseMaxHPAction,
        assert_matches,
        card::CardRef,
        combat::EndTurnStep,
        game::{Game, GameBuilder},
        potion::Potion,
        relic::RelicClass,
    };

    use super::*;

    #[test]
    fn test_divider() {
        {
            let mut g = GameBuilder::default().build_combat_with_monster(Hexaghost::new());
            g.player.cur_hp = 1;
            g.step_test(EndTurnStep);
            assert_matches!(g.monsters[0].behavior.get_intent(), Intent::Attack(1, 6));
            g.throw_potion(Potion::Blood, None);
            assert_matches!(g.monsters[0].behavior.get_intent(), Intent::Attack(1, 6));
        }
        {
            let mut g = GameBuilder::default().build_combat_with_monster(Hexaghost::new());
            g.player.cur_hp = 23;
            g.step_test(EndTurnStep);
            assert_matches!(g.monsters[0].behavior.get_intent(), Intent::Attack(2, 6));
        }
        {
            let mut g = GameBuilder::default().build_combat_with_monster(Hexaghost::new());
            g.player.cur_hp = 24;
            g.step_test(EndTurnStep);
            assert_matches!(g.monsters[0].behavior.get_intent(), Intent::Attack(3, 6));
        }
    }

    #[test]
    fn test_basic() {
        let assert_discard_pile = |g: &Game, class: CardClass| {
            assert!(g.discard_pile.iter().all(|c| c.borrow().class == class));
        };
        let mut g = GameBuilder::default()
            .add_cards(CardClass::Strike, 10)
            .add_relic(RelicClass::RunicPyramid)
            .build_combat_with_monster(Hexaghost::new());
        g.run_action(IncreaseMaxHPAction(9999));
        assert_matches!(g.monsters[0].behavior.get_intent(), Intent::Unknown);
        // activate
        g.step_test(EndTurnStep);
        // divider
        g.step_test(EndTurnStep);
        // sear
        g.step_test(EndTurnStep);
        assert_eq!(g.discard_pile.len(), 2);
        assert_discard_pile(&g, CardClass::Burn);
        // tackle
        g.step_test(EndTurnStep);
        assert_eq!(g.discard_pile.len(), 2);
        // sear
        g.step_test(EndTurnStep);
        assert_eq!(g.discard_pile.len(), 4);
        // inflame
        g.step_test(EndTurnStep);
        assert_eq!(g.discard_pile.len(), 4);
        // tackle
        g.step_test(EndTurnStep);
        assert_eq!(g.discard_pile.len(), 4);
        // sear
        g.step_test(EndTurnStep);
        assert_eq!(g.discard_pile.len(), 6);
        assert_discard_pile(&g, CardClass::Burn);
        // inferno
        g.step_test(EndTurnStep);
        assert_eq!(g.discard_pile.len(), 9);
        assert_discard_pile(&g, CardClass::BurnPlus);
        // sear
        g.step_test(EndTurnStep);
        assert_eq!(g.discard_pile.len(), 11);
        assert_discard_pile(&g, CardClass::BurnPlus);
    }

    #[test]
    fn test_upgrade_burns() {
        let mut g = GameBuilder::default()
            .add_player_status(Status::DarkEmbrace, 1)
            .build_combat_with_monster(Hexaghost::new());
        g.run_action(IncreaseMaxHPAction(9999));

        g.step_test(EndTurnStep);
        g.step_test(EndTurnStep);
        g.step_test(EndTurnStep);
        g.step_test(EndTurnStep);
        g.step_test(EndTurnStep);
        g.step_test(EndTurnStep);
        g.step_test(EndTurnStep);
        g.step_test(EndTurnStep);
        // inferno
        g.add_card_to_hand(CardClass::Dazed);
        g.add_card_to_exhaust_pile(CardClass::Burn);
        g.step_test(EndTurnStep);
        let mut upgraded_count = 0;
        let mut not_upgraded_count = 0;
        let mut count = |c: &CardRef| match c.borrow().class {
            CardClass::Burn => not_upgraded_count += 1,
            CardClass::BurnPlus => upgraded_count += 1,
            _ => {}
        };
        for c in g.draw_pile.get_all() {
            count(c);
        }
        for c in &g.hand {
            count(c);
        }
        for c in &g.discard_pile {
            count(c);
        }
        for c in &g.exhaust_pile {
            count(c);
        }
        assert_eq!(upgraded_count, 8);
        assert_eq!(not_upgraded_count, 2);
    }
}
