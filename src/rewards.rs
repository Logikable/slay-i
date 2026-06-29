use crate::{
    actions::{
        add_card_to_master_deck::AddCardToMasterDeckAction, gain_gold::GainGoldAction,
        gain_potion::GainPotionAction, gain_relic::GainRelicAction,
        increase_max_hp::IncreaseMaxHPAction,
    },
    card::CardRef,
    cards::{CardRarity, random_common_red, random_rare_red, random_uncommon_red},
    game::{EnterActGameState, Game, RareCardBaseChance, RunActionsGameState},
    potion::Potion,
    relic::{RelicClass, RelicRarity},
    state::{GameState, Steps},
    step::Step,
};

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum RewardType {
    Monster,
    Elite,
    Boss,
    Mushrooms,
    DeadAdventurer {
        gold_reward: bool,
        relic_reward: bool,
    },
}

#[derive(Default, Debug, Clone)]
pub struct Rewards {
    pub gold: i32,
    pub stolen_gold: i32,
    pub potions: Vec<Potion>,
    pub cards: Vec<Vec<CardRef>>,
    pub relics: Vec<RelicClass>,
    pub has_sapphire_key: bool,
}

impl Rewards {
    pub fn gen_card_reward(game: &mut Game, ty: RareCardBaseChance) -> Vec<CardRef> {
        let mut num = 3;
        if game.has_relic(RelicClass::BustedCrown) {
            num -= 2;
        }
        if game.has_relic(RelicClass::QuestionCard) {
            num += 1;
        }
        let mut cards = Vec::<CardRef>::new();
        for _ in 0..num {
            let rarity = game.roll_rarity(ty);
            match rarity {
                CardRarity::Common => game.rare_card_chance += 1,
                CardRarity::Uncommon => {}
                CardRarity::Rare => game.rare_card_chance = 0,
                CardRarity::Basic | CardRarity::Special | CardRarity::Curse => panic!(),
            };
            let mut class;
            loop {
                class = match rarity {
                    CardRarity::Common => random_common_red(&mut game.rng),
                    CardRarity::Uncommon => random_uncommon_red(&mut game.rng),
                    CardRarity::Rare => random_rare_red(&mut game.rng),
                    CardRarity::Basic | CardRarity::Special | CardRarity::Curse => panic!(),
                };
                if cards.iter().all(|c| c.borrow().class != class) {
                    break;
                }
            }
            cards.push(game.new_card(class));
        }
        cards
    }
}

impl Rewards {
    pub fn add_gold(&mut self, mut amount: i32, has_golden_idol: bool) {
        if has_golden_idol {
            amount += (amount as f32 * 0.25).round() as i32
        }
        self.gold += amount;
    }
    pub fn add_stolen_gold(&mut self, amount: i32) {
        self.stolen_gold += amount;
    }
    pub fn add_potion(&mut self, potion: Potion) {
        self.potions.push(potion);
    }
    pub fn add_cards(&mut self, cards: Vec<CardRef>) {
        self.cards.push(cards);
    }
    pub fn add_relic(&mut self, relic: RelicClass) {
        self.relics.push(relic);
    }
    pub fn remove_one_relic(&mut self) {
        self.relics.remove(0);
    }
}

#[derive(Debug)]
pub struct RewardsGameState;

impl GameState for RewardsGameState {
    fn valid_steps(&self, game: &Game) -> Option<Steps> {
        let mut steps = Steps::default();
        if !game.has_relic(RelicClass::Ectoplasm) {
            if game.rewards.gold != 0 {
                steps.push(GoldRewardStep);
            }
            if game.rewards.stolen_gold != 0 {
                steps.push(StolenGoldRewardStep);
            }
        }
        if !game.has_relic(RelicClass::Sozu) {
            for i in 0..game.rewards.potions.len() {
                steps.push(PotionRewardStep { potion_index: i });
            }
        }
        for (i, cards) in game.rewards.cards.iter().enumerate() {
            for ci in 0..cards.len() {
                steps.push(CardRewardStep {
                    pack_index: i,
                    card_index: ci,
                });
            }
            if game.has_relic(RelicClass::SingingBowl) {
                steps.push(SingingBowlStep { pack_index: i });
            }
        }
        for i in 0..game.rewards.relics.len() {
            steps.push(RelicRewardStep { relic_index: i });
        }
        if game.rewards.has_sapphire_key && !game.rewards.relics.is_empty() {
            steps.push(SapphireKeyStep);
        }
        steps.push(RewardExitStep);
        Some(steps)
    }
}

#[derive(Debug, PartialEq, Eq)]
struct GoldRewardStep;

impl Step for GoldRewardStep {
    fn should_pop_state(&self) -> bool {
        false
    }
    fn run(&self, game: &mut Game) {
        game.action_queue
            .push_bot(GainGoldAction(game.rewards.gold));
        game.rewards.gold = 0;
        game.state.push_state(RunActionsGameState);
    }
    fn description(&self, game: &Game) -> String {
        format!("gain {} gold", game.rewards.gold)
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct StolenGoldRewardStep;

impl Step for StolenGoldRewardStep {
    fn should_pop_state(&self) -> bool {
        false
    }
    fn run(&self, game: &mut Game) {
        game.action_queue
            .push_bot(GainGoldAction(game.rewards.stolen_gold));
        game.rewards.stolen_gold = 0;
        game.state.push_state(RunActionsGameState);
    }
    fn description(&self, game: &Game) -> String {
        format!("gain {} gold", game.rewards.stolen_gold)
    }
}

#[derive(Debug, PartialEq, Eq)]
struct PotionRewardStep {
    potion_index: usize,
}

impl Step for PotionRewardStep {
    fn should_pop_state(&self) -> bool {
        false
    }
    fn run(&self, game: &mut Game) {
        let p = game.rewards.potions.remove(self.potion_index);
        game.action_queue.push_bot(GainPotionAction(p));
        game.state.push_state(RunActionsGameState);
    }
    fn description(&self, game: &Game) -> String {
        format!("gain {:?} potion", game.rewards.potions[self.potion_index])
    }
}

#[derive(Debug, PartialEq, Eq)]
struct CardRewardStep {
    pack_index: usize,
    card_index: usize,
}

impl Step for CardRewardStep {
    fn should_pop_state(&self) -> bool {
        false
    }
    fn run(&self, game: &mut Game) {
        let mut pack = game.rewards.cards.remove(self.pack_index);
        let c = pack.remove(self.card_index);
        game.action_queue.push_bot(AddCardToMasterDeckAction(c));
        game.state.push_state(RunActionsGameState);
    }
    fn description(&self, game: &Game) -> String {
        let all = game.rewards.cards[self.pack_index]
            .iter()
            .map(|c| format!("{:?}", c.borrow()))
            .collect::<Vec<_>>()
            .join(", ");
        format!(
            "gain {:?} out of {}",
            game.rewards.cards[self.pack_index][self.card_index].borrow(),
            all
        )
    }
}

#[derive(Debug, PartialEq, Eq)]
struct SingingBowlStep {
    pack_index: usize,
}

impl Step for SingingBowlStep {
    fn should_pop_state(&self) -> bool {
        false
    }
    fn run(&self, game: &mut Game) {
        game.rewards.cards.remove(self.pack_index);
        game.action_queue.push_bot(IncreaseMaxHPAction(2));
        game.state.push_state(RunActionsGameState);
    }
    fn description(&self, game: &Game) -> String {
        let all = game.rewards.cards[self.pack_index]
            .iter()
            .map(|c| format!("{:?}", c.borrow()))
            .collect::<Vec<_>>()
            .join(", ");
        format!("skip for +2 max hp out of {}", all)
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct RelicRewardStep {
    pub relic_index: usize,
}

impl Step for RelicRewardStep {
    fn should_pop_state(&self) -> bool {
        false
    }
    fn run(&self, game: &mut Game) {
        if game.rewards.has_sapphire_key && self.relic_index + 1 == game.rewards.relics.len() {
            game.rewards.has_sapphire_key = false;
        }
        let r = game.rewards.relics.remove(self.relic_index);
        game.action_queue.push_bot(GainRelicAction(r));
        game.state.push_state(RunActionsGameState);
    }
    fn description(&self, game: &Game) -> String {
        format!("gain {:?}", game.rewards.relics[self.relic_index])
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct SapphireKeyStep;

impl Step for SapphireKeyStep {
    fn should_pop_state(&self) -> bool {
        false
    }
    fn run(&self, game: &mut Game) {
        game.has_sapphire_key = true;
        game.rewards.has_sapphire_key = false;
        game.rewards.relics.pop().unwrap();
    }
    fn description(&self, game: &Game) -> String {
        format!(
            "gain sapphire key (lose {:?})",
            game.rewards.relics.last().unwrap()
        )
    }
}

#[derive(Eq, PartialEq, Debug)]
pub struct RewardExitStep;

impl Step for RewardExitStep {
    fn should_pop_state(&self) -> bool {
        true
    }
    fn run(&self, game: &mut Game) {
        game.rewards = Rewards::default();
    }

    fn description(&self, _: &Game) -> String {
        "exit".to_owned()
    }
}

#[derive(Debug)]
pub struct BossRewardGameState;

impl GameState for BossRewardGameState {
    fn run(&self, game: &mut Game) {
        for _ in 0..3 {
            let r = game.next_relic(RelicRarity::Boss);
            game.boss_rewards.push(r);
        }
    }
    fn valid_steps(&self, game: &Game) -> Option<Steps> {
        let mut steps = Steps::default();
        for i in 0..game.boss_rewards.len() {
            steps.push(BossRewardChooseStep {
                boss_reward_index: i,
            });
        }
        steps.push(BossRewardSkipStep);
        Some(steps)
    }
}

#[derive(Debug, PartialEq, Eq)]
struct BossRewardChooseStep {
    boss_reward_index: usize,
}

impl Step for BossRewardChooseStep {
    fn should_pop_state(&self) -> bool {
        true
    }
    fn run(&self, game: &mut Game) {
        game.state.push_state(EnterActGameState);

        let r = game.boss_rewards[self.boss_reward_index];
        game.boss_rewards.clear();
        game.action_queue.push_bot(GainRelicAction(r));
        game.state.push_state(RunActionsGameState);
    }
    fn description(&self, game: &Game) -> String {
        format!("choose {:?}", game.boss_rewards[self.boss_reward_index])
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct BossRewardSkipStep;

impl Step for BossRewardSkipStep {
    fn should_pop_state(&self) -> bool {
        true
    }
    fn run(&self, game: &mut Game) {
        game.state.push_state(EnterActGameState);
        game.boss_rewards.clear();
    }
    fn description(&self, _: &Game) -> String {
        "skip".to_owned()
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        assert_matches,
        campfire::CampfireRestStep,
        cards::CardClass,
        chest::OpenChestStep,
        combat::EndTurnStep,
        game::{AscendStep, DiscardPotionStep, GameBuilder},
        map::RoomType,
        monster::Intent,
        monsters::looter::Looter,
        relic::RelicRarity,
    };

    use super::*;

    #[test]
    fn test_combat_rewards() {
        let mut g = GameBuilder::default().build_combat();
        g.potion_chance = 0;
        g.play_card(CardClass::DebugKillAll, None);
        assert_ne!(g.rewards.gold, 0);
        assert_eq!(g.rewards.stolen_gold, 0);
        assert_eq!(g.rewards.cards.len(), 1);
        assert_eq!(g.rewards.cards[0].len(), 3);
        assert_eq!(
            g.valid_steps(),
            vec![
                Box::new(GoldRewardStep) as Box<dyn Step>,
                Box::new(CardRewardStep {
                    pack_index: 0,
                    card_index: 0
                }),
                Box::new(CardRewardStep {
                    pack_index: 0,
                    card_index: 1
                }),
                Box::new(CardRewardStep {
                    pack_index: 0,
                    card_index: 2
                }),
                Box::new(RewardExitStep),
            ]
        );
        let gold = g.rewards.gold;
        g.step_test(GoldRewardStep);
        assert_eq!(g.gold, gold);
        assert!(gold >= 10);
        assert!(gold <= 20);
        let class = g.rewards.cards[0][0].borrow().class;
        g.step_test(CardRewardStep {
            pack_index: 0,
            card_index: 0,
        });
        assert_eq!(g.master_deck[0].borrow().class, class);
        assert_eq!(
            g.valid_steps(),
            vec![Box::new(RewardExitStep) as Box<dyn Step>]
        );
    }

    #[test]
    fn test_elite_rewards() {
        for _ in 0..10 {
            let mut g = GameBuilder::default().build_with_rooms(&[RoomType::Elite]);
            g.potion_chance = 0;
            g.step_test(AscendStep::new(0, 0));
            g.play_card(CardClass::DebugKillAll, None);
            assert_ne!(g.rewards.gold, 0);
            assert_eq!(g.rewards.stolen_gold, 0);
            assert_eq!(g.rewards.cards.len(), 1);
            assert_eq!(g.rewards.cards[0].len(), 3);
            assert_eq!(g.rewards.relics.len(), 1);
            assert_eq!(
                g.valid_steps(),
                vec![
                    Box::new(GoldRewardStep) as Box<dyn Step>,
                    Box::new(CardRewardStep {
                        pack_index: 0,
                        card_index: 0
                    }),
                    Box::new(CardRewardStep {
                        pack_index: 0,
                        card_index: 1
                    }),
                    Box::new(CardRewardStep {
                        pack_index: 0,
                        card_index: 2
                    }),
                    Box::new(RelicRewardStep { relic_index: 0 }),
                    Box::new(RewardExitStep),
                ]
            );
            let gold = g.rewards.gold;
            g.step_test(GoldRewardStep);
            assert_eq!(g.gold, gold);
            assert!(gold >= 25);
            assert!(gold <= 35);
            g.step_test(RelicRewardStep { relic_index: 0 });
            assert_eq!(g.relics.len(), 1);
            assert_matches!(
                g.relics[0].get_class().rarity(),
                RelicRarity::Common | RelicRarity::Uncommon | RelicRarity::Rare
            );
        }
    }

    #[test]
    fn test_potion_reward() {
        let mut g = GameBuilder::default().build_combat();
        g.potion_chance = 100;
        g.play_card(CardClass::DebugKillAll, None);
        g.step_test(PotionRewardStep { potion_index: 0 });
        assert!(g.potions[0].is_some());
    }

    #[test]
    fn test_stolen_gold() {
        let mut g = GameBuilder::default().build_combat();
        g.rewards.stolen_gold = 100;
        g.play_card(CardClass::DebugKillAll, None);
        g.step_test(StolenGoldRewardStep);
        assert_eq!(g.gold, 100);
    }

    #[test]
    fn test_white_beast_statue() {
        let mut g = GameBuilder::default()
            .add_relic(RelicClass::WhiteBeastStatue)
            .build_combat();
        g.play_card(CardClass::DebugKillAll, None);
        g.step_test(PotionRewardStep { potion_index: 0 });
        assert!(g.potions[0].is_some());
    }

    #[test]
    fn test_rare() {
        let mut found_rare = false;
        for _ in 0..1000 {
            let mut g =
                GameBuilder::default().build_with_rooms(&[RoomType::Monster, RoomType::Monster]);
            g.roll_noop_monsters = true;
            g.step_test(AscendStep::new(0, 0));
            g.play_card(CardClass::DebugKillAll, None);
            assert!(
                g.rewards
                    .cards
                    .iter()
                    .flatten()
                    .all(|c| c.borrow().class.rarity() != CardRarity::Rare)
            );
            g.step_test(RewardExitStep);
            g.step_test(AscendStep::new(0, 1));
            g.play_card(CardClass::DebugKillAll, None);
            found_rare = g
                .rewards
                .cards
                .iter()
                .flatten()
                .any(|c| c.borrow().class.rarity() == CardRarity::Rare);
            if found_rare {
                break;
            }
        }
        assert!(found_rare);
    }

    #[test]
    fn test_nloths_gift() {
        let mut found_rare = false;
        for _ in 0..50 {
            let mut g = GameBuilder::default()
                .add_relic(RelicClass::NlothsGift)
                .build_combat();
            g.play_card(CardClass::DebugKillAll, None);
            found_rare = g
                .rewards
                .cards
                .iter()
                .flatten()
                .any(|c| c.borrow().class.rarity() == CardRarity::Rare);
            if found_rare {
                break;
            }
        }
        assert!(found_rare);
    }

    #[test]
    fn test_escape_rewards() {
        let mut g = GameBuilder::default().build_combat_with_monster(Looter::new());
        g.potion_chance = 100;
        loop {
            let done = matches!(g.monsters[0].behavior.get_intent(), Intent::Escape);
            g.step_test(EndTurnStep);
            if done {
                break;
            }
        }
        assert_eq!(g.rewards.gold, 0);
        assert!(!g.rewards.cards.is_empty());
        assert!(g.rewards.potions.is_empty());
    }

    #[test]
    fn test_smoke_bomb_rewards() {
        let mut g = GameBuilder::default().build_combat();
        g.throw_potion(Potion::Smoke, None);
        assert_eq!(
            g.valid_steps(),
            vec![Box::new(RewardExitStep) as Box<dyn Step>]
        );
    }

    #[test]
    fn test_campfire_dreamcatcher() {
        let mut g = GameBuilder::default()
            .add_relic(RelicClass::DreamCatcher)
            .build_campfire();
        g.step_test(CampfireRestStep);
        assert_eq!(
            g.valid_steps(),
            vec![
                Box::new(CardRewardStep {
                    pack_index: 0,
                    card_index: 0
                }) as Box<dyn Step>,
                Box::new(CardRewardStep {
                    pack_index: 0,
                    card_index: 1
                }),
                Box::new(CardRewardStep {
                    pack_index: 0,
                    card_index: 2
                }),
                Box::new(RewardExitStep)
            ]
        );
    }

    #[test]
    fn test_golden_idol() {
        for _ in 0..10 {
            let mut g = GameBuilder::default()
                .add_relic(RelicClass::GoldenIdol)
                .build_with_rooms(&[RoomType::Monster, RoomType::Elite]);
            g.step_test(AscendStep::new(0, 0));
            g.play_card(CardClass::DebugKillAll, None);
            assert!(g.rewards.gold >= 13);
            assert!(g.rewards.gold <= 25);
            g.step_test(RewardExitStep);

            g.step_test(AscendStep::new(0, 1));
            g.play_card(CardClass::DebugKillAll, None);
            assert!(g.rewards.gold >= 31);
            assert!(g.rewards.gold <= 44);
        }
        {
            let mut g = GameBuilder::default()
                .add_relic(RelicClass::GoldenIdol)
                .build_combat_with_monster(Looter::new());
            g.gold = 100;
            g.step_test(EndTurnStep);
            g.play_card(CardClass::DebugKillAll, None);
            assert_eq!(g.rewards.stolen_gold, 20);
        }
    }

    #[test]
    fn test_singing_bowl() {
        let mut g = GameBuilder::default()
            .add_relic(RelicClass::SingingBowl)
            .build_combat();
        let max_hp = g.player.max_hp;
        g.play_card(CardClass::DebugKillAll, None);
        g.step_test(SingingBowlStep { pack_index: 0 });
        assert_eq!(g.player.max_hp, max_hp + 2);
        assert!(g.rewards.cards.is_empty());
    }

    #[test]
    fn test_prayer_wheel() {
        let mut g = GameBuilder::default()
            .add_relic(RelicClass::PrayerWheel)
            .build_with_rooms(&[RoomType::Monster, RoomType::Elite]);
        g.step_test(AscendStep::new(0, 0));
        g.play_card(CardClass::DebugKillAll, None);
        assert_eq!(g.rewards.cards.len(), 2);
        g.step_test(CardRewardStep {
            pack_index: 0,
            card_index: 0,
        });
        g.step_test(CardRewardStep {
            pack_index: 0,
            card_index: 0,
        });
        g.step_test(RewardExitStep);
        g.step_test(AscendStep::new(0, 1));
        g.play_card(CardClass::DebugKillAll, None);
        assert_eq!(g.rewards.cards.len(), 1);
    }

    #[test]
    fn test_orrery() {
        let mut g = GameBuilder::default().build_shop();
        g.run_action(GainRelicAction(RelicClass::Orrery));
        assert_eq!(g.rewards.cards.len(), 5);
        for _ in 0..5 {
            g.step_test(CardRewardStep {
                pack_index: 0,
                card_index: 1,
            });
        }
    }

    #[test]
    fn test_cauldron() {
        let mut g = GameBuilder::default().build_shop();
        g.run_action(GainRelicAction(RelicClass::Cauldron));
        assert_eq!(g.rewards.potions.len(), 5);
        assert!(g.rewards.cards.is_empty());
        assert!(g.rewards.relics.is_empty());
        for _ in 0..5 {
            g.step_test(PotionRewardStep { potion_index: 0 });
            g.step_test(DiscardPotionStep { potion_index: 0 });
        }
    }

    #[test]
    fn test_boss_rewards() {
        let mut g = GameBuilder::default().build_with_rooms(&[RoomType::BossTreasure]);
        g.step_test(AscendStep::new(0, 0));
        assert_eq!(
            g.valid_steps(),
            vec![
                Box::new(BossRewardChooseStep {
                    boss_reward_index: 0
                }) as Box<dyn Step>,
                Box::new(BossRewardChooseStep {
                    boss_reward_index: 1
                }),
                Box::new(BossRewardChooseStep {
                    boss_reward_index: 2
                }),
                Box::new(BossRewardSkipStep),
            ]
        );
        let r = g.boss_rewards[0];
        g.step_test(BossRewardChooseStep {
            boss_reward_index: 0,
        });
        assert!(g.has_relic(r));
    }

    #[test]
    fn test_calling_bell() {
        let mut g = GameBuilder::default().build_shop();
        g.run_action(GainRelicAction(RelicClass::CallingBell));
        assert_eq!(g.rewards.relics.len(), 3);
        assert_eq!(g.rewards.potions.len(), 0);
        assert_eq!(g.rewards.cards.len(), 0);
        assert_eq!(g.master_deck.len(), 1);
        assert_eq!(g.master_deck[0].borrow().class, CardClass::CurseOfTheBell);
        assert_eq!(g.rewards.relics[0].rarity(), RelicRarity::Common);
        assert_eq!(g.rewards.relics[1].rarity(), RelicRarity::Uncommon);
        assert_eq!(g.rewards.relics[2].rarity(), RelicRarity::Rare);
    }

    #[test]
    fn test_black_star() {
        for _ in 0..10 {
            let mut g = GameBuilder::default()
                .add_relic(RelicClass::BlackStar)
                .build_with_rooms(&[RoomType::Elite]);
            g.potion_chance = 0;
            g.step_test(AscendStep::new(0, 0));
            g.play_card(CardClass::DebugKillAll, None);
            assert_eq!(g.rewards.relics.len(), 2);
            assert!(!g.rewards.relics[1].is_campfire_relic());
        }
    }

    #[test]
    fn test_tiny_house() {
        let mut g = GameBuilder::default()
            .add_cards(CardClass::Strike, 2)
            .build();
        g.run_action(GainRelicAction(RelicClass::TinyHouse));
        assert_eq!(
            g.master_deck
                .iter()
                .map(|c| c.borrow().upgrade_count)
                .sum::<i32>(),
            1
        );
        assert_eq!(g.rewards.gold, 50);
        assert_eq!(g.rewards.stolen_gold, 0);
        assert_eq!(g.rewards.cards.len(), 1);
        assert_eq!(g.rewards.potions.len(), 1);
        assert_eq!(g.rewards.cards[0].len(), 3);
    }

    #[test]
    fn test_sapphire_key_1() {
        let mut g =
            GameBuilder::default().build_with_rooms(&[RoomType::Treasure, RoomType::Treasure]);
        g.step_test(AscendStep::new(0, 0));
        g.step_test(OpenChestStep);
        g.rewards.gold = 0;
        assert!(!g.has_sapphire_key);
        assert_eq!(
            g.valid_steps(),
            vec![
                Box::new(RelicRewardStep { relic_index: 0 }) as Box<dyn Step>,
                Box::new(SapphireKeyStep),
                Box::new(RewardExitStep),
            ]
        );
        g.step_test(SapphireKeyStep);
        assert!(g.has_sapphire_key);
        g.step_test(RewardExitStep);
        g.step_test(AscendStep::new(0, 1));
        g.step_test(OpenChestStep);
        g.rewards.gold = 0;
        assert_eq!(
            g.valid_steps(),
            vec![
                Box::new(RelicRewardStep { relic_index: 0 }) as Box<dyn Step>,
                Box::new(RewardExitStep),
            ]
        );
    }

    #[test]
    fn test_sapphire_key_2() {
        let mut g =
            GameBuilder::default().build_with_rooms(&[RoomType::Treasure, RoomType::Treasure]);
        g.step_test(AscendStep::new(0, 0));
        g.step_test(OpenChestStep);
        g.rewards.gold = 0;
        assert!(!g.has_sapphire_key);
        g.rewards.relics[0] = RelicClass::Akabeko;
        assert_eq!(
            g.valid_steps(),
            vec![
                Box::new(RelicRewardStep { relic_index: 0 }) as Box<dyn Step>,
                Box::new(SapphireKeyStep),
                Box::new(RewardExitStep),
            ]
        );
        g.step_test(RelicRewardStep { relic_index: 0 });
        assert!(!g.has_sapphire_key);
        assert_eq!(
            g.valid_steps(),
            vec![Box::new(RewardExitStep) as Box<dyn Step>,]
        );
    }

    #[test]
    fn test_sapphire_key_3() {
        let mut g =
            GameBuilder::default().build_with_rooms(&[RoomType::Treasure, RoomType::Treasure]);
        g.step_test(AscendStep::new(0, 0));
        g.step_test(OpenChestStep);
        g.rewards.gold = 0;
        g.rewards.relics[0] = RelicClass::Akabeko;
        g.rewards.add_relic(RelicClass::Anchor);
        assert_eq!(
            g.valid_steps(),
            vec![
                Box::new(RelicRewardStep { relic_index: 0 }) as Box<dyn Step>,
                Box::new(RelicRewardStep { relic_index: 1 }),
                Box::new(SapphireKeyStep),
                Box::new(RewardExitStep),
            ]
        );
        g.step_test(RelicRewardStep { relic_index: 0 });
        assert_eq!(
            g.valid_steps(),
            vec![
                Box::new(RelicRewardStep { relic_index: 0 }) as Box<dyn Step>,
                Box::new(SapphireKeyStep),
                Box::new(RewardExitStep),
            ]
        );
        assert!(!g.has_sapphire_key);
    }

    #[test]
    fn test_sapphire_key_4() {
        let mut g =
            GameBuilder::default().build_with_rooms(&[RoomType::Treasure, RoomType::Treasure]);
        g.step_test(AscendStep::new(0, 0));
        g.step_test(OpenChestStep);
        g.rewards.gold = 0;
        g.rewards.relics[0] = RelicClass::Akabeko;
        g.rewards.add_relic(RelicClass::Anchor);
        assert_eq!(g.rewards.relics.len(), 2);
        assert_eq!(
            g.valid_steps(),
            vec![
                Box::new(RelicRewardStep { relic_index: 0 }) as Box<dyn Step>,
                Box::new(RelicRewardStep { relic_index: 1 }),
                Box::new(SapphireKeyStep),
                Box::new(RewardExitStep),
            ]
        );
        g.step_test(RelicRewardStep { relic_index: 1 });
        assert_eq!(
            g.valid_steps(),
            vec![
                Box::new(RelicRewardStep { relic_index: 0 }) as Box<dyn Step>,
                Box::new(RewardExitStep),
            ]
        );
        assert!(!g.has_sapphire_key);
    }

    #[test]
    fn test_nloths_hungry_face() {
        let mut g = GameBuilder::default()
            .add_relic(RelicClass::NlothsHungryFace)
            .build_with_rooms(&[RoomType::Treasure, RoomType::Treasure]);
        g.step_test(AscendStep::new(0, 0));
        assert_eq!(g.get_relic_value(RelicClass::NlothsHungryFace), Some(1));
        g.step_test(OpenChestStep);
        assert_eq!(g.get_relic_value(RelicClass::NlothsHungryFace), Some(0));
        g.rewards.gold = 0;
        assert_eq!(g.rewards.relics.len(), 0);
        assert_eq!(
            g.valid_steps(),
            vec![Box::new(RewardExitStep) as Box<dyn Step>,]
        );
        g.step_test(RewardExitStep);
        g.step_test(AscendStep::new(0, 1));
        g.step_test(OpenChestStep);
        assert_eq!(g.rewards.relics.len(), 1);
    }

    #[test]
    fn test_nloths_hungry_face_matryoshka() {
        let mut g = GameBuilder::default()
            .add_relic(RelicClass::NlothsHungryFace)
            .add_relic(RelicClass::Matryoshka)
            .build_with_rooms(&[RoomType::Treasure]);
        g.step_test(AscendStep::new(0, 0));
        g.step_test(OpenChestStep);
        g.rewards.gold = 0;
        assert_eq!(g.rewards.relics.len(), 1);
        assert_eq!(
            g.valid_steps(),
            vec![
                Box::new(RelicRewardStep { relic_index: 0 }) as Box<dyn Step>,
                Box::new(SapphireKeyStep),
                Box::new(RewardExitStep),
            ]
        );
    }
}
