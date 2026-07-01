use crate::{
    actions::{
        add_card_class_to_master_deck::AddCardClassToMasterDeckAction, gain_gold::GainGoldAction,
        gain_potion::GainPotionAction, gain_relic::GainRelicAction,
        increase_max_hp::IncreaseMaxHPAction,
    },
    cards::random_rare_red,
    game::{Game, RunActionsGameState},
    master_deck::{
        ChooseRemoveFromMasterGameState, ChooseTransformMasterGameState,
        ChooseUpgradeMasterGameState,
    },
    potion::random_common_potion,
    relic::{RelicClass, RelicRarity},
    rng::{Rand, rand_slice},
    state::{GameState, Steps},
    step::Step,
};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Blessing {
    RemoveCard,
    UpgradeCard,
    TransformCard,
    RandomRareCard,
    ThreePotions,
    CommonRelic,
    TenPercentMaxHp,
    NeowsLament,
    HundredGold,
}

const CARD_BLESSINGS: [Blessing; 4] = [
    Blessing::RemoveCard,
    Blessing::UpgradeCard,
    Blessing::TransformCard,
    Blessing::RandomRareCard,
];

const BONUS_BLESSINGS: [Blessing; 5] = [
    Blessing::ThreePotions,
    Blessing::CommonRelic,
    Blessing::TenPercentMaxHp,
    Blessing::NeowsLament,
    Blessing::HundredGold,
];

impl Blessing {
    pub fn roll(rng: &mut Rand) -> Vec<Blessing> {
        vec![
            rand_slice(rng, &CARD_BLESSINGS),
            rand_slice(rng, &BONUS_BLESSINGS),
        ]
    }

    pub fn run(&self, game: &mut Game) {
        use Blessing::*;
        match self {
            RemoveCard => {
                game.state.push_state(ChooseRemoveFromMasterGameState {
                    num_cards_remaining: 1,
                });
            }
            UpgradeCard => {
                game.state.push_state(ChooseUpgradeMasterGameState);
            }
            TransformCard => {
                game.state.push_state(ChooseTransformMasterGameState {
                    num_cards_remaining: 1,
                    upgrade: false,
                });
            }
            RandomRareCard => {
                let c = random_rare_red(&mut game.rng);
                game.action_queue
                    .push_bot(AddCardClassToMasterDeckAction(c));
            }
            ThreePotions => {
                let free = game.potions.iter().filter(|p| p.is_none()).count();
                for _ in 0..free.min(3) {
                    let p = random_common_potion(&mut game.rng);
                    game.action_queue.push_bot(GainPotionAction(p));
                }
            }
            CommonRelic => {
                let r = game.next_relic(RelicRarity::Common);
                game.action_queue.push_bot(GainRelicAction(r));
            }
            TenPercentMaxHp => {
                let amount = (game.player.max_hp as f32 * 0.1) as i32;
                game.action_queue.push_bot(IncreaseMaxHPAction(amount));
            }
            NeowsLament => {
                game.action_queue
                    .push_bot(GainRelicAction(RelicClass::NeowsLament));
            }
            HundredGold => {
                game.action_queue.push_bot(GainGoldAction(100));
            }
        }
    }
}

#[derive(Debug)]
pub struct ChooseBlessingGameState {
    pub rewards: Vec<Blessing>,
}

impl GameState for ChooseBlessingGameState {
    fn valid_steps(&self, _: &Game) -> Option<Steps> {
        let mut steps = Steps::default();
        for &b in &self.rewards {
            steps.push(ChooseBlessingStep(b));
        }
        Some(steps)
    }
}

#[derive(Eq, PartialEq, Debug)]
pub struct ChooseBlessingStep(pub Blessing);

impl Step for ChooseBlessingStep {
    fn should_pop_state(&self) -> bool {
        true
    }

    fn run(&self, game: &mut Game) {
        self.0.run(game);
        game.state.push_state(RunActionsGameState);
    }

    fn description(&self, _: &Game) -> String {
        format!("{:?}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::{
        BONUS_BLESSINGS, Blessing, CARD_BLESSINGS, ChooseBlessingGameState, ChooseBlessingStep,
    };
    use crate::{
        game::GameBuilder, master_deck::ChooseRemoveFromMasterStep, relic::RelicClass, rng::Rand,
    };

    fn build_with_blessing(b: Blessing) -> crate::game::Game {
        GameBuilder::default().build_with_game_state(ChooseBlessingGameState { rewards: vec![b] })
    }

    #[test]
    fn test_roll_offers_one_card_and_one_bonus_blessing() {
        let mut rng = Rand::seed_from_u64(0);
        let rewards = Blessing::roll(&mut rng);
        assert_eq!(rewards.len(), 2);
        assert!(CARD_BLESSINGS.contains(&rewards[0]));
        assert!(BONUS_BLESSINGS.contains(&rewards[1]));
    }

    #[test]
    fn test_hundred_gold_blessing_adds_100_gold() {
        let mut g = build_with_blessing(Blessing::HundredGold);
        let gold = g.gold;
        g.step_test(ChooseBlessingStep(Blessing::HundredGold));
        assert_eq!(g.gold, gold + 100);
    }

    #[test]
    fn test_ten_percent_max_hp_blessing_adds_a_tenth_of_max_hp() {
        let mut g = build_with_blessing(Blessing::TenPercentMaxHp);
        let max_hp = g.player.max_hp;
        g.step_test(ChooseBlessingStep(Blessing::TenPercentMaxHp));
        assert_eq!(g.player.max_hp, max_hp + (max_hp as f32 * 0.1) as i32);
    }

    #[test]
    fn test_neows_lament_blessing_grants_the_relic() {
        let mut g = build_with_blessing(Blessing::NeowsLament);
        g.step_test(ChooseBlessingStep(Blessing::NeowsLament));
        assert!(g.has_relic(RelicClass::NeowsLament));
    }

    #[test]
    fn test_three_potions_blessing_fills_the_potion_slots() {
        let mut g = build_with_blessing(Blessing::ThreePotions);
        g.step_test(ChooseBlessingStep(Blessing::ThreePotions));
        assert!(g.potions.iter().all(|p| p.is_some()));
    }

    #[test]
    fn test_remove_card_blessing_shrinks_the_master_deck() {
        let mut g = GameBuilder::default()
            .ironclad_starting_deck()
            .build_with_game_state(ChooseBlessingGameState {
                rewards: vec![Blessing::RemoveCard],
            });
        let size = g.master_deck.len();
        g.step_test(ChooseBlessingStep(Blessing::RemoveCard));
        g.step_test(ChooseRemoveFromMasterStep {
            master_index: 0,
            num_cards_remaining: 1,
        });
        assert_eq!(g.master_deck.len(), size - 1);
    }
}
