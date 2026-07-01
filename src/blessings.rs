use crate::{
    actions::{
        add_card_class_to_master_deck::AddCardClassToMasterDeckAction, damage::DamageAction,
        decrease_max_hp::DecreaseMaxHPAction, gain_gold::GainGoldAction,
        gain_potion::GainPotionAction, gain_relic::GainRelicAction,
        increase_max_hp::IncreaseMaxHPAction, remove_relic::RemoveRelicAction,
    },
    card::CardRef,
    cards::{random_curse, random_rare_colorless, random_rare_red, random_uncommon_colorless},
    game::{Game, RareCardBaseChance, RunActionsGameState},
    master_deck::{
        ChooseRemoveFromMasterGameState, ChooseTransformMasterGameState,
        ChooseUpgradeMasterGameState,
    },
    potion::random_common_potion,
    relic::{RelicClass, RelicRarity},
    rewards::{Rewards, RewardsGameState},
    rng::{Rand, rand_slice},
    state::{GameState, Steps},
    step::Step,
};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Blessing {
    ThreeCards,
    RandomColorless,
    RemoveCard,
    UpgradeCard,
    TransformCard,
    RandomRareCard,
    ThreePotions,
    CommonRelic,
    TenPercentMaxHp,
    NeowsLament,
    HundredGold,
    Composite(Drawback, CompositeReward),
    BossRelic,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Drawback {
    TenPercentHpLoss,
    NoGold,
    Curse,
    PercentDamage,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CompositeReward {
    RandomColorless2,
    RemoveTwo,
    OneRareRelic,
    ThreeRareCards,
    TwoFiftyGold,
    TransformTwoCards,
    TwentyPercentHpBonus,
}

const DRAWBACKS: [Drawback; 4] = [
    Drawback::TenPercentHpLoss,
    Drawback::NoGold,
    Drawback::Curse,
    Drawback::PercentDamage,
];

const CARD_BLESSINGS: [Blessing; 6] = [
    Blessing::ThreeCards,
    Blessing::RandomColorless,
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

fn compatible_rewards(drawback: Drawback) -> Vec<CompositeReward> {
    use CompositeReward::*;
    let mut rewards = vec![
        RandomColorless2,
        OneRareRelic,
        ThreeRareCards,
        TransformTwoCards,
    ];
    if drawback != Drawback::Curse {
        rewards.push(RemoveTwo);
    }
    if drawback != Drawback::NoGold {
        rewards.push(TwoFiftyGold);
    }
    if drawback != Drawback::TenPercentHpLoss {
        rewards.push(TwentyPercentHpBonus);
    }
    rewards
}

fn colorless_pack(game: &mut Game, rare: bool) -> Vec<CardRef> {
    let mut cards: Vec<CardRef> = Vec::new();
    while cards.len() < 3 {
        let class = if rare {
            random_rare_colorless(&mut game.rng)
        } else {
            random_uncommon_colorless(&mut game.rng)
        };
        if cards.iter().all(|c| c.borrow().class != class) {
            cards.push(game.new_card(class));
        }
    }
    cards
}

impl Blessing {
    pub fn roll(rng: &mut Rand) -> Vec<Blessing> {
        let drawback = rand_slice(rng, &DRAWBACKS);
        vec![
            rand_slice(rng, &CARD_BLESSINGS),
            rand_slice(rng, &BONUS_BLESSINGS),
            Blessing::Composite(drawback, rand_slice(rng, &compatible_rewards(drawback))),
            Blessing::BossRelic,
        ]
    }

    pub fn run(&self, game: &mut Game) {
        use Blessing::*;
        match self {
            ThreeCards => {
                let cards = Rewards::gen_card_reward(game, RareCardBaseChance::Normal);
                game.rewards.add_cards(cards);
                game.state.push_state(RewardsGameState);
            }
            RandomColorless => {
                let cards = colorless_pack(game, false);
                game.rewards.add_cards(cards);
                game.state.push_state(RewardsGameState);
            }
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
            Composite(drawback, reward) => {
                drawback.run(game);
                reward.run(game);
            }
            BossRelic => {
                let starter = game.relics[0].get_class();
                game.action_queue.push_bot(RemoveRelicAction(starter));
                let r = game.next_relic(RelicRarity::Boss);
                game.action_queue.push_bot(GainRelicAction(r));
            }
        }
    }
}

impl Drawback {
    fn run(&self, game: &mut Game) {
        use Drawback::*;
        match self {
            TenPercentHpLoss => {
                let amount = (game.player.max_hp as f32 * 0.1) as i32;
                game.action_queue.push_bot(DecreaseMaxHPAction(amount));
            }
            NoGold => {
                game.gold = 0;
            }
            Curse => {
                let c = random_curse(&mut game.rng);
                game.action_queue
                    .push_bot(AddCardClassToMasterDeckAction(c));
            }
            PercentDamage => {
                let amount = game.player.cur_hp / 10 * 3;
                game.action_queue.push_bot(DamageAction::event(amount));
            }
        }
    }
}

impl CompositeReward {
    fn run(&self, game: &mut Game) {
        use CompositeReward::*;
        match self {
            RandomColorless2 => {
                let cards = colorless_pack(game, true);
                game.rewards.add_cards(cards);
                game.state.push_state(RewardsGameState);
            }
            RemoveTwo => {
                game.state.push_state(ChooseRemoveFromMasterGameState {
                    num_cards_remaining: 2,
                });
            }
            OneRareRelic => {
                let r = game.next_relic(RelicRarity::Rare);
                game.action_queue.push_bot(GainRelicAction(r));
            }
            ThreeRareCards => {
                let cards = Rewards::gen_card_reward(game, RareCardBaseChance::Boss);
                game.rewards.add_cards(cards);
                game.state.push_state(RewardsGameState);
            }
            TwoFiftyGold => {
                game.action_queue.push_bot(GainGoldAction(250));
            }
            TransformTwoCards => {
                game.state.push_state(ChooseTransformMasterGameState {
                    num_cards_remaining: 2,
                    upgrade: false,
                });
            }
            TwentyPercentHpBonus => {
                let amount = (game.player.max_hp as f32 * 0.1) as i32 * 2;
                game.action_queue.push_bot(IncreaseMaxHPAction(amount));
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
        CompositeReward, Drawback, compatible_rewards,
    };
    use crate::{
        cards::CardType, game::GameBuilder, master_deck::ChooseRemoveFromMasterStep,
        relic::RelicClass, rng::Rand,
    };

    fn build_with_blessing(b: Blessing) -> crate::game::Game {
        GameBuilder::default().build_with_game_state(ChooseBlessingGameState { rewards: vec![b] })
    }

    fn curse_count(g: &crate::game::Game) -> usize {
        g.master_deck
            .iter()
            .filter(|c| c.borrow().class.ty() == CardType::Curse)
            .count()
    }

    #[test]
    fn test_roll_offers_a_card_a_bonus_a_composite_and_a_boss_relic_blessing() {
        let mut rng = Rand::seed_from_u64(0);
        let rewards = Blessing::roll(&mut rng);
        assert_eq!(rewards.len(), 4);
        assert!(CARD_BLESSINGS.contains(&rewards[0]));
        assert!(BONUS_BLESSINGS.contains(&rewards[1]));
        assert!(matches!(rewards[2], Blessing::Composite(..)));
        assert_eq!(rewards[3], Blessing::BossRelic);
    }

    #[test]
    fn test_boss_relic_blessing_replaces_the_starter_relic() {
        let mut g = GameBuilder::default()
            .add_relic(RelicClass::BurningBlood)
            .build_with_game_state(ChooseBlessingGameState {
                rewards: vec![Blessing::BossRelic],
            });
        g.step_test(ChooseBlessingStep(Blessing::BossRelic));
        assert!(!g.has_relic(RelicClass::BurningBlood));
        assert_eq!(g.relics.len(), 1);
    }

    #[test]
    fn test_composite_rewards_exclude_the_forbidden_pairings() {
        assert!(!compatible_rewards(Drawback::Curse).contains(&CompositeReward::RemoveTwo));
        assert!(!compatible_rewards(Drawback::NoGold).contains(&CompositeReward::TwoFiftyGold));
        assert!(
            !compatible_rewards(Drawback::TenPercentHpLoss)
                .contains(&CompositeReward::TwentyPercentHpBonus)
        );
    }

    fn build_and_apply_composite(drawback: Drawback, reward: CompositeReward) -> crate::game::Game {
        let mut g = build_with_blessing(Blessing::Composite(drawback, reward));
        g.gold = 250;
        g.step_test(ChooseBlessingStep(Blessing::Composite(drawback, reward)));
        g
    }

    #[test]
    fn test_composite_no_gold_drawback_zeroes_gold() {
        let g = build_and_apply_composite(Drawback::NoGold, CompositeReward::TwentyPercentHpBonus);
        assert_eq!(g.gold, 0);
    }

    #[test]
    fn test_composite_one_rare_relic_reward_grants_a_relic() {
        let g = build_and_apply_composite(Drawback::NoGold, CompositeReward::OneRareRelic);
        assert_eq!(g.relics.len(), 1);
    }

    #[test]
    fn test_composite_hp_loss_drawback_lowers_max_hp_and_grants_gold() {
        let mut g = build_with_blessing(Blessing::Composite(
            Drawback::TenPercentHpLoss,
            CompositeReward::TwoFiftyGold,
        ));
        let max_hp = g.player.max_hp;
        let gold = g.gold;
        g.step_test(ChooseBlessingStep(Blessing::Composite(
            Drawback::TenPercentHpLoss,
            CompositeReward::TwoFiftyGold,
        )));
        assert_eq!(g.player.max_hp, max_hp - (max_hp as f32 * 0.1) as i32);
        assert_eq!(g.gold, gold + 250);
    }

    #[test]
    fn test_composite_curse_drawback_adds_a_curse_card() {
        let mut g = GameBuilder::default()
            .ironclad_starting_deck()
            .build_with_game_state(ChooseBlessingGameState {
                rewards: vec![Blessing::Composite(
                    Drawback::Curse,
                    CompositeReward::TwoFiftyGold,
                )],
            });
        let curses = curse_count(&g);
        g.step_test(ChooseBlessingStep(Blessing::Composite(
            Drawback::Curse,
            CompositeReward::TwoFiftyGold,
        )));
        assert_eq!(curse_count(&g), curses + 1);
    }

    #[test]
    fn test_composite_percent_damage_drawback_loses_current_hp() {
        let mut g = build_with_blessing(Blessing::Composite(
            Drawback::PercentDamage,
            CompositeReward::TwoFiftyGold,
        ));
        let cur_hp = g.player.cur_hp;
        g.step_test(ChooseBlessingStep(Blessing::Composite(
            Drawback::PercentDamage,
            CompositeReward::TwoFiftyGold,
        )));
        assert_eq!(g.player.cur_hp, cur_hp - cur_hp / 10 * 3);
    }

    #[test]
    fn test_composite_remove_two_reward_removes_two_cards() {
        let mut g = GameBuilder::default()
            .ironclad_starting_deck()
            .build_with_game_state(ChooseBlessingGameState {
                rewards: vec![Blessing::Composite(
                    Drawback::NoGold,
                    CompositeReward::RemoveTwo,
                )],
            });
        let size = g.master_deck.len();
        g.step_test(ChooseBlessingStep(Blessing::Composite(
            Drawback::NoGold,
            CompositeReward::RemoveTwo,
        )));
        g.step_test(ChooseRemoveFromMasterStep {
            master_index: 0,
            num_cards_remaining: 2,
        });
        g.step_test(ChooseRemoveFromMasterStep {
            master_index: 0,
            num_cards_remaining: 1,
        });
        assert_eq!(g.master_deck.len(), size - 2);
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
    fn test_three_cards_blessing_adds_a_card_to_the_master_deck() {
        let mut g = GameBuilder::default()
            .ironclad_starting_deck()
            .build_with_game_state(ChooseBlessingGameState {
                rewards: vec![Blessing::ThreeCards],
            });
        let size = g.master_deck.len();
        g.step_test(ChooseBlessingStep(Blessing::ThreeCards));
        g.step(0);
        assert_eq!(g.master_deck.len(), size + 1);
    }

    #[test]
    fn test_random_colorless_blessing_adds_an_uncommon_colorless_card() {
        use crate::cards::{CardColor, CardRarity};
        let mut g = GameBuilder::default()
            .ironclad_starting_deck()
            .build_with_game_state(ChooseBlessingGameState {
                rewards: vec![Blessing::RandomColorless],
            });
        let size = g.master_deck.len();
        g.step_test(ChooseBlessingStep(Blessing::RandomColorless));
        g.step(0);
        assert_eq!(g.master_deck.len(), size + 1);
        let added = g.master_deck.last().unwrap().borrow();
        assert_eq!(added.class.color(), CardColor::Colorless);
        assert_eq!(added.class.rarity(), CardRarity::Uncommon);
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
