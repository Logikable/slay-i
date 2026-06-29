use rand::RngExt;

use crate::{
    actions::{
        add_card_class_to_master_deck::AddCardClassToMasterDeckAction,
        gain_potion::GainPotionAction, gain_relic::GainRelicAction,
    },
    cards::{
        CardClass, CardColor, CardRarity, CardType, random_rare_colorless, random_red_attack,
        random_red_power, random_red_skill, random_uncommon_colorless,
    },
    game::{Game, RareCardBaseChance, RunActionsGameState},
    master_deck::ChooseRemoveFromMasterGameState,
    potion::{
        Potion, PotionRarity, random_common_potion, random_rare_potion, random_uncommon_potion,
    },
    relic::{RelicClass, RelicRarity},
    state::{GameState, Steps},
    step::Step,
};

fn break_maw_bank(game: &mut Game) {
    if game.has_relic(RelicClass::MawBank) {
        game.set_relic_value(RelicClass::MawBank, 0);
    }
}

#[derive(Default, Clone)]
pub struct Shop {
    pub cards: Vec<(CardClass, i32)>,
    pub relics: Vec<(RelicClass, i32)>,
    pub potions: Vec<(Potion, i32)>,
    pub can_remove: bool,
}

impl Shop {
    pub fn new(game: &mut Game) -> Self {
        let mut shop = Self::default();
        for card_f in [
            random_red_attack,
            random_red_attack,
            random_red_skill,
            random_red_skill,
            random_red_power,
        ] {
            let mut class;
            loop {
                let rarity = game.roll_rarity(RareCardBaseChance::Shop);
                class = card_f(&mut game.rng);
                if class.rarity() == rarity && shop.cards.iter().all(|(c, _)| *c != class) {
                    break;
                }
            }
            shop.cards
                .push((class, Self::base_card_cost(game, class, true)));
        }
        for card_f in [random_uncommon_colorless, random_rare_colorless] {
            let class = card_f(&mut game.rng);
            shop.cards
                .push((class, Self::base_card_cost(game, class, true)));
        }
        // sale
        let discount_i = game.rng.random_range(0..5);
        shop.cards[discount_i].1 /= 2;

        for _ in 0..3 {
            let potion = Self::random_potion(game);
            shop.potions
                .push((potion, Self::base_potion_cost(game, potion, true)));
        }

        for _ in 0..2 {
            let relic = Self::random_non_shop_relic(game);
            shop.relics
                .push((relic, Self::base_relic_cost(game, relic, true)));
        }
        {
            let relic = game.next_relic(RelicRarity::Shop);
            shop.relics
                .push((relic, Self::base_relic_cost(game, relic, true)));
        }

        shop.can_remove = true;

        shop
    }

    pub fn apply_discount(price: &mut i32, mult: f32) {
        *price = ((*price as f32) * mult).round() as i32;
    }

    fn discount_item(mut price: i32, game: &Game) -> i32 {
        if game.has_relic(RelicClass::TheCourier) {
            Self::apply_discount(&mut price, 0.8);
        }
        if game.has_relic(RelicClass::MembershipCard) {
            Self::apply_discount(&mut price, 0.5);
        }
        price
    }

    fn get_card(&self, i: usize, game: &Game) -> (CardClass, i32) {
        let (c, price) = self.cards[i];
        (c, Self::discount_item(price, game))
    }

    fn get_potion(&self, i: usize, game: &Game) -> (Potion, i32) {
        let (p, price) = self.potions[i];
        (p, Self::discount_item(price, game))
    }

    fn get_relic(&self, i: usize, game: &Game) -> (RelicClass, i32) {
        let (r, price) = self.relics[i];
        (r, Self::discount_item(price, game))
    }

    fn random_potion(game: &mut Game) -> Potion {
        // 65% common
        // 25% uncommon
        // 10% rare
        match game.rng.random_range(0..100) {
            0..50 => random_common_potion(&mut game.rng),
            50..90 => random_uncommon_potion(&mut game.rng),
            _ => random_rare_potion(&mut game.rng),
        }
    }

    fn random_non_shop_relic(game: &mut Game) -> RelicClass {
        // 50% common
        // 33% uncommon
        // 17% rare
        loop {
            let rarity = match game.rng.random_range(0..100) {
                0..50 => RelicRarity::Common,
                50..83 => RelicRarity::Uncommon,
                _ => RelicRarity::Rare,
            };
            let r = game.next_relic(rarity);
            if !matches!(
                r,
                RelicClass::OldCoin
                    | RelicClass::TheCourier
                    | RelicClass::SmilingMask
                    | RelicClass::MawBank
            ) {
                return r;
            }
        }
    }

    fn price_variance(game: &mut Game) -> f32 {
        game.rng.random_range(0.95..=1.05)
    }

    fn restock_card(prev_card: CardClass, game: &mut Game) -> CardClass {
        if prev_card.color() == CardColor::Colorless {
            match game.rng.random_range(0..10) {
                0..3 => random_rare_colorless(&mut game.rng),
                _ => random_uncommon_colorless(&mut game.rng),
            }
        } else {
            let card_f = match prev_card.ty() {
                CardType::Attack => random_red_attack,
                CardType::Skill => random_red_skill,
                CardType::Power => random_red_power,
                _ => panic!(),
            };
            loop {
                let rarity = game.roll_rarity(RareCardBaseChance::Shop);
                let class = card_f(&mut game.rng);
                if class.rarity() == rarity {
                    return class;
                }
            }
        }
    }

    fn base_card_cost(game: &mut Game, class: CardClass, ascension_discount: bool) -> i32 {
        let mut price = match class.rarity() {
            CardRarity::Common => 50,
            CardRarity::Uncommon => 75,
            CardRarity::Rare => 150,
            CardRarity::Basic | CardRarity::Special | CardRarity::Curse => panic!(),
        };
        if class.color() == CardColor::Colorless {
            price = (price as f32 * 1.2).floor() as i32;
        }
        Self::apply_discount(&mut price, Self::price_variance(game));
        if ascension_discount {
            Self::apply_discount(&mut price, 1.1);
        }
        price
    }

    fn base_potion_cost(game: &mut Game, potion: Potion, ascension_discount: bool) -> i32 {
        let mut price = match potion.rarity() {
            PotionRarity::Common => 50,
            PotionRarity::Uncommon => 75,
            PotionRarity::Rare => 100,
        };
        Self::apply_discount(&mut price, Self::price_variance(game));
        if ascension_discount {
            Self::apply_discount(&mut price, 1.1);
        }
        price
    }

    fn base_relic_cost(game: &mut Game, relic: RelicClass, ascension_discount: bool) -> i32 {
        let mut price = match relic.rarity() {
            RelicRarity::Common | RelicRarity::Shop => 150,
            RelicRarity::Uncommon => 250,
            RelicRarity::Rare => 300,
            _ => panic!(),
        };
        Self::apply_discount(&mut price, Self::price_variance(game));
        if ascension_discount {
            Self::apply_discount(&mut price, 1.1);
        }
        price
    }

    fn remove_cost(game: &Game) -> i32 {
        if game.has_relic(RelicClass::SmilingMask) {
            return 50;
        }
        let mut price = 75 + game.shop_remove_count * 25;
        if game.has_relic(RelicClass::TheCourier) {
            Self::apply_discount(&mut price, 0.8);
        }
        if game.has_relic(RelicClass::MembershipCard) {
            Self::apply_discount(&mut price, 0.5);
        }
        price
    }
}

#[derive(Debug)]
pub struct ShopGameState;

impl GameState for ShopGameState {
    fn valid_steps(&self, game: &Game) -> Option<Steps> {
        let mut steps = Steps::default();
        for (i, (_, price)) in game.shop.cards.iter().enumerate() {
            if game.gold >= *price {
                steps.push(ShopBuyCardStep { shop_index: i });
            }
        }
        if !game.has_relic(RelicClass::Sozu) {
            for (i, (_, price)) in game.shop.potions.iter().enumerate() {
                if game.gold >= *price {
                    steps.push(ShopBuyPotionStep { shop_index: i });
                }
            }
        }
        for (i, (_, price)) in game.shop.relics.iter().enumerate() {
            if game.gold >= *price {
                steps.push(ShopBuyRelicStep { shop_index: i });
            }
        }
        if game.shop.can_remove && game.gold >= Shop::remove_cost(game) {
            steps.push(ShopRemoveCardStep);
        }
        steps.push(ShopExitStep);
        Some(steps)
    }
}

#[derive(Debug, PartialEq, Eq)]
struct ShopBuyCardStep {
    shop_index: usize,
}

impl Step for ShopBuyCardStep {
    fn should_pop_state(&self) -> bool {
        false
    }

    fn run(&self, game: &mut Game) {
        break_maw_bank(game);

        let (class, price) = game.shop.get_card(self.shop_index, game);
        if game.has_relic(RelicClass::TheCourier) {
            let new_card = Shop::restock_card(class, game);
            game.shop.cards[self.shop_index] =
                (new_card, Shop::base_card_cost(game, new_card, false));
        } else {
            game.shop.cards.remove(self.shop_index);
        }
        game.gold -= price;
        game.action_queue
            .push_bot(AddCardClassToMasterDeckAction(class));
        game.state.push_state(RunActionsGameState);
    }
    fn description(&self, game: &Game) -> String {
        let (class, price) = game.shop.get_card(self.shop_index, game);
        format!("buy {:?} for {}", class, price)
    }
}

#[derive(Debug, PartialEq, Eq)]
struct ShopBuyPotionStep {
    shop_index: usize,
}

impl Step for ShopBuyPotionStep {
    fn should_pop_state(&self) -> bool {
        false
    }

    fn run(&self, game: &mut Game) {
        break_maw_bank(game);

        let (potion, price) = game.shop.get_potion(self.shop_index, game);
        if game.has_relic(RelicClass::TheCourier) {
            let new_potion = Shop::random_potion(game);
            game.shop.potions[self.shop_index] =
                (new_potion, Shop::base_potion_cost(game, new_potion, false));
        } else {
            game.shop.potions.remove(self.shop_index);
        }
        game.gold -= price;
        game.action_queue.push_bot(GainPotionAction(potion));
        game.state.push_state(RunActionsGameState);
    }
    fn description(&self, game: &Game) -> String {
        let (potion, price) = game.shop.get_potion(self.shop_index, game);
        format!("buy {:?} for {}", potion, price)
    }
}

#[derive(Debug, PartialEq, Eq)]
struct ShopBuyRelicStep {
    shop_index: usize,
}

impl Step for ShopBuyRelicStep {
    fn should_pop_state(&self) -> bool {
        false
    }

    fn run(&self, game: &mut Game) {
        break_maw_bank(game);

        let (relic, price) = game.shop.get_relic(self.shop_index, game);
        if game.has_relic(RelicClass::TheCourier) {
            let new_relic = Shop::random_non_shop_relic(game);
            game.shop.relics[self.shop_index] =
                (new_relic, Shop::base_relic_cost(game, new_relic, false));
        } else {
            game.shop.relics.remove(self.shop_index);
        }
        game.gold -= price;
        game.action_queue.push_bot(GainRelicAction(relic));
        game.state.push_state(RunActionsGameState);
    }
    fn description(&self, game: &Game) -> String {
        let (relic, price) = game.shop.get_relic(self.shop_index, game);
        format!("buy {:?} for {}", relic, price)
    }
}

#[derive(Debug, PartialEq, Eq)]
struct ShopRemoveCardStep;

impl Step for ShopRemoveCardStep {
    fn should_pop_state(&self) -> bool {
        false
    }

    fn run(&self, game: &mut Game) {
        break_maw_bank(game);

        let price = Shop::remove_cost(game);
        game.gold -= price;
        game.shop_remove_count += 1;
        game.shop.can_remove = false;
        game.state.push_state(ChooseRemoveFromMasterGameState {
            num_cards_remaining: 1,
        });
    }
    fn description(&self, game: &Game) -> String {
        format!("remove card for {}", Shop::remove_cost(game))
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct ShopExitStep;

impl Step for ShopExitStep {
    fn should_pop_state(&self) -> bool {
        true
    }
    fn run(&self, game: &mut Game) {
        game.shop = Shop::default();
    }
    fn description(&self, _: &Game) -> String {
        "exit shop".to_owned()
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        assert_matches, assert_not_matches,
        cards::{CardClass, CardColor, CardRarity, CardType},
        game::{AscendStep, GameBuilder},
        map::RoomType,
        master_deck::ChooseRemoveFromMasterStep,
        potion::{Potion, PotionRarity},
        relic::{RelicClass, RelicRarity},
        rewards::RewardExitStep,
        shop::{
            ShopBuyCardStep, ShopBuyPotionStep, ShopBuyRelicStep, ShopExitStep, ShopRemoveCardStep,
        },
        step::Step,
    };

    #[test]
    fn test_shop_prices() {
        for _ in 0..50 {
            let g = GameBuilder::default().build_shop();
            assert_eq!(g.shop.cards.len(), 7);
            let mut count_discount = 0;
            for i in 0..5 {
                let (c, price) = g.shop.get_card(i, &g);
                assert_eq!(c.color(), CardColor::Red);
                match i {
                    0 | 1 => assert_eq!(c.ty(), CardType::Attack),
                    2 | 3 => assert_eq!(c.ty(), CardType::Skill),
                    4 => assert_eq!(c.ty(), CardType::Power),
                    _ => panic!(),
                }
                let (lo, hi) = match c.rarity() {
                    CardRarity::Common => (50, 61),
                    CardRarity::Uncommon => (75, 91),
                    CardRarity::Rare => (149, 182),
                    _ => panic!(),
                };
                if price < lo {
                    count_discount += 1;
                } else {
                    assert!(price >= lo);
                    assert!(price <= hi);
                }
            }
            assert_eq!(count_discount, 1);
            for i in 5..7 {
                let (c, price) = g.shop.get_card(i, &g);
                assert_eq!(c.color(), CardColor::Colorless);
                if i == 5 {
                    assert_eq!(c.rarity(), CardRarity::Uncommon);
                } else {
                    assert_eq!(c.rarity(), CardRarity::Rare);
                }
                let (lo, hi) = match c.rarity() {
                    CardRarity::Uncommon => (89, 109),
                    CardRarity::Rare => (178, 218),
                    _ => panic!(),
                };
                assert!(price >= lo);
                assert!(price <= hi);
            }
            assert_eq!(g.shop.potions.len(), 3);
            for i in 0..3 {
                let (potion, price) = g.shop.get_potion(i, &g);
                let (lo, hi) = match potion.rarity() {
                    PotionRarity::Common => (52, 58),
                    PotionRarity::Uncommon => (78, 87),
                    PotionRarity::Rare => (105, 116),
                };
                assert!(price >= lo);
                assert!(price <= hi);
            }
            assert_eq!(g.shop.relics.len(), 3);
            assert_matches!(
                g.shop.relics[0].0.rarity(),
                RelicRarity::Common | RelicRarity::Uncommon | RelicRarity::Rare
            );
            assert_matches!(
                g.shop.relics[1].0.rarity(),
                RelicRarity::Common | RelicRarity::Uncommon | RelicRarity::Rare
            );
            assert_matches!(g.shop.relics[2].0.rarity(), RelicRarity::Shop);
            for i in 0..3 {
                let (relic, price) = g.shop.get_relic(i, &g);
                let (lo, hi) = match relic.rarity() {
                    RelicRarity::Common | RelicRarity::Shop => (157, 173),
                    RelicRarity::Uncommon => (261, 289),
                    RelicRarity::Rare => (314, 347),
                    _ => panic!(),
                };
                assert!(price >= lo);
                assert!(price <= hi);
            }
        }
    }

    #[test]
    fn test_restock_prices() {
        for _ in 0..50 {
            let mut g = GameBuilder::default()
                .add_relic(RelicClass::TheCourier)
                .build_shop();
            g.gold = 2000;
            for i in 0..7 {
                g.step_test(ShopBuyCardStep { shop_index: i });
            }
            for i in 0..3 {
                g.step_test(ShopBuyPotionStep { shop_index: i });
                g.take_potion(0);
            }
            g.shop.relics[0].0 = RelicClass::Akabeko;
            g.shop.relics[1].0 = RelicClass::Anchor;
            g.shop.relics[2].0 = RelicClass::AncientTeaSet;
            for i in 0..3 {
                g.step_test(ShopBuyRelicStep { shop_index: i });
            }
            // we may end up buying membership card
            g.relics.clear();
            g.add_relic(RelicClass::TheCourier);

            assert_eq!(g.shop.cards.len(), 7);
            let discounted = |p: i32| (p as f32 * 0.8).round() as i32;
            for i in 0..5 {
                let (c, price) = g.shop.get_card(i, &g);
                assert_eq!(c.color(), CardColor::Red);
                match i {
                    0 | 1 => assert_eq!(c.ty(), CardType::Attack),
                    2 | 3 => assert_eq!(c.ty(), CardType::Skill),
                    4 => assert_eq!(c.ty(), CardType::Power),
                    _ => panic!(),
                }
                let (lo, hi) = match c.rarity() {
                    CardRarity::Common => (45, 55),
                    CardRarity::Uncommon => (68, 83),
                    CardRarity::Rare => (135, 165),
                    _ => panic!(),
                };
                assert!(price >= discounted(lo));
                assert!(price <= discounted(hi));
            }
            for i in 5..7 {
                let (c, price) = g.shop.get_card(i, &g);
                assert_eq!(c.color(), CardColor::Colorless);
                let (lo, hi) = match c.rarity() {
                    CardRarity::Uncommon => (81, 99),
                    CardRarity::Rare => (162, 198),
                    _ => panic!(),
                };
                assert!(price >= discounted(lo));
                assert!(price <= discounted(hi));
            }
            assert_eq!(g.shop.potions.len(), 3);
            for i in 0..3 {
                let (potion, price) = g.shop.get_potion(i, &g);
                let (lo, hi) = match potion.rarity() {
                    PotionRarity::Common => (48, 53),
                    PotionRarity::Uncommon => (71, 79),
                    PotionRarity::Rare => (95, 105),
                };
                assert!(price >= discounted(lo));
                assert!(price <= discounted(hi));
            }
            assert_eq!(g.shop.relics.len(), 3);
            for i in 0..3 {
                let (relic, price) = g.shop.get_relic(i, &g);
                assert_matches!(
                    relic.rarity(),
                    RelicRarity::Common | RelicRarity::Uncommon | RelicRarity::Rare
                );
                let (lo, hi) = match relic.rarity() {
                    RelicRarity::Common => (143, 158),
                    RelicRarity::Uncommon => (238, 263),
                    RelicRarity::Rare => (285, 315),
                    _ => panic!(),
                };
                assert!(price >= discounted(lo));
                assert!(price <= discounted(hi));
            }
        }
    }

    #[test]
    fn test_payment() {
        let mut g = GameBuilder::default().build_shop();

        g.gold = 1000;
        g.shop.cards[0] = (CardClass::WildStrike, 100);
        g.step_test(ShopBuyCardStep { shop_index: 0 });
        assert_eq!(g.shop.cards.len(), 6);
        assert_eq!(g.gold, 1000 - 100);
        assert_eq!(g.master_deck.len(), 1);
        assert_eq!(g.master_deck[0].borrow().class, CardClass::WildStrike);

        g.gold = 1000;
        g.shop.potions[0] = (Potion::Fire, 50);
        g.step_test(ShopBuyPotionStep { shop_index: 0 });
        assert_eq!(g.shop.potions.len(), 2);
        assert_eq!(g.potions, vec![Some(Potion::Fire), None]);
        assert_eq!(g.gold, 1000 - 50);

        g.gold = 1000;
        g.shop.relics[0] = (RelicClass::CentennialPuzzle, 150);
        g.step_test(ShopBuyRelicStep { shop_index: 0 });
        assert!(g.has_relic(RelicClass::CentennialPuzzle));
        assert_eq!(g.shop.relics.len(), 2);
        assert_eq!(g.gold, 1000 - 150);
    }

    #[test]
    fn test_payment_membership_card() {
        let mut g = GameBuilder::default().build_shop();

        g.add_relic(RelicClass::MembershipCard);

        g.gold = 1000;
        g.shop.cards[0] = (CardClass::WildStrike, 100);
        g.step_test(ShopBuyCardStep { shop_index: 0 });
        assert_eq!(g.gold, 1000 - 50);

        g.gold = 1000;
        g.shop.potions[0] = (Potion::Fire, 50);
        g.step_test(ShopBuyPotionStep { shop_index: 0 });
        assert_eq!(g.gold, 1000 - 25);

        g.gold = 1000;
        g.shop.relics[0] = (RelicClass::CentennialPuzzle, 150);
        g.step_test(ShopBuyRelicStep { shop_index: 0 });
        assert!(g.has_relic(RelicClass::CentennialPuzzle));
        assert_eq!(g.gold, 1000 - 75);
    }

    #[test]
    fn test_payment_courier() {
        let mut g = GameBuilder::default()
            .add_relic(RelicClass::TheCourier)
            .build_shop();

        g.gold = 1000;
        g.shop.cards[0] = (CardClass::WildStrike, 100);
        g.step_test(ShopBuyCardStep { shop_index: 0 });
        assert_eq!(g.gold, 1000 - (100_f32 * 0.8).round() as i32);

        g.gold = 1000;
        g.shop.potions[0] = (Potion::Fire, 50);
        g.step_test(ShopBuyPotionStep { shop_index: 0 });
        assert_eq!(g.gold, 1000 - (50_f32 * 0.8).round() as i32);

        g.gold = 1000;
        g.shop.relics[0] = (RelicClass::CentennialPuzzle, 150);
        g.step_test(ShopBuyRelicStep { shop_index: 0 });
        assert!(g.has_relic(RelicClass::CentennialPuzzle));
        assert_eq!(g.gold, 1000 - (150_f32 * 0.8).round() as i32);
    }

    #[test]
    fn test_remove() {
        let mut g = GameBuilder::default()
            .add_cards(CardClass::Strike, 10)
            .build_with_rooms(&[
                RoomType::Shop,
                RoomType::Shop,
                RoomType::Shop,
                RoomType::Shop,
            ]);
        g.step_test(AscendStep::new(0, 0));
        g.gold = 1000;
        g.step_test(ShopRemoveCardStep);
        g.step_test(ChooseRemoveFromMasterStep {
            master_index: 0,
            num_cards_remaining: 1,
        });
        assert!(
            !g.valid_steps()
                .contains(&(Box::new(ShopRemoveCardStep) as Box<dyn Step>))
        );
        assert_eq!(g.gold, 1000 - 75);

        g.step_test(ShopExitStep);
        g.step_test(AscendStep::new(0, 1));
        g.step_test(ShopRemoveCardStep);
        g.step_test(ChooseRemoveFromMasterStep {
            master_index: 0,
            num_cards_remaining: 1,
        });
        assert_eq!(g.gold, 1000 - 75 - 100);

        g.add_relic(RelicClass::MembershipCard);
        g.step_test(ShopExitStep);
        g.step_test(AscendStep::new(0, 2));
        g.step_test(ShopRemoveCardStep);
        g.step_test(ChooseRemoveFromMasterStep {
            master_index: 0,
            num_cards_remaining: 1,
        });
        assert_eq!(g.gold, 1000 - 75 - 100 - ((125.0_f32 * 0.5).round() as i32));

        g.add_relic(RelicClass::TheCourier);
        g.step_test(ShopExitStep);
        g.step_test(AscendStep::new(0, 3));
        g.step_test(ShopRemoveCardStep);
        g.step_test(ChooseRemoveFromMasterStep {
            master_index: 0,
            num_cards_remaining: 1,
        });
        assert_eq!(
            g.gold,
            1000 - 75
                - 100
                - ((125.0_f32 * 0.5).round() as i32)
                - ((150.0_f32 * 0.4).round() as i32)
        );
    }

    #[test]
    fn test_expensive() {
        let mut g = GameBuilder::default().build_shop();
        g.gold = 500;
        g.shop.can_remove = false;
        for i in 0..6 {
            g.shop.cards[i].1 = 1000;
        }
        g.shop.cards[6].1 = 100;

        g.shop.potions[0].1 = 1000;
        g.shop.potions[1].1 = 1000;
        g.shop.potions[2].1 = 100;

        g.shop.relics[0].1 = 1000;
        g.shop.relics[1].1 = 1000;
        g.shop.relics[2].1 = 100;

        assert_eq!(
            g.valid_steps(),
            vec![
                Box::new(ShopBuyCardStep { shop_index: 6 }) as Box<dyn Step>,
                Box::new(ShopBuyPotionStep { shop_index: 2 }),
                Box::new(ShopBuyRelicStep { shop_index: 2 }),
                Box::new(ShopExitStep),
            ]
        );
    }

    #[test]
    fn test_remove_expensive() {
        let mut g = GameBuilder::default().build_shop();
        g.gold = 75;
        g.shop.cards.clear();
        g.shop.potions.clear();
        g.shop.relics.clear();
        assert_eq!(
            g.valid_steps(),
            vec![
                Box::new(ShopRemoveCardStep) as Box<dyn Step>,
                Box::new(ShopExitStep),
            ]
        );
        g.gold = 74;
        assert_eq!(
            g.valid_steps(),
            vec![Box::new(ShopExitStep) as Box<dyn Step>,]
        );
    }

    #[test]
    fn test_maw_bank() {
        let mut g = GameBuilder::default()
            .add_relic(RelicClass::MawBank)
            .build_with_rooms(&[
                RoomType::Monster,
                RoomType::Shop,
                RoomType::Shop,
                RoomType::Monster,
            ]);
        g.gold = 500;
        g.step_test(AscendStep::new(0, 0));
        assert_eq!(g.gold, 512);
        g.play_card(CardClass::DebugKillAll, None);
        g.step_test(RewardExitStep);
        g.step_test(AscendStep::new(0, 1));
        assert_eq!(g.gold, 524);
        g.step_test(ShopExitStep);
        g.step_test(AscendStep::new(0, 2));
        assert_eq!(g.gold, 536);
        assert_eq!(g.get_relic_value(RelicClass::MawBank), Some(1));
        g.step_test(ShopBuyPotionStep { shop_index: 0 });
        assert_eq!(g.get_relic_value(RelicClass::MawBank), Some(0));
        g.gold = 500;
        g.step_test(ShopExitStep);
        g.step_test(AscendStep::new(0, 3));
        assert_eq!(g.gold, 500);
    }

    #[test]
    fn test_shop_rare_chance() {
        for _ in 0..10 {
            let mut g = GameBuilder::default().build_with_rooms(&[RoomType::Shop]);
            g.rare_card_chance = -10;
            g.step_test(AscendStep::new(0, 0));
            assert!(!g.shop.cards.is_empty());
            for i in 0..5 {
                assert_ne!(g.shop.cards[i].0.rarity(), CardRarity::Rare);
            }
        }
        let mut found_rare = false;
        for _ in 0..100 {
            let mut g = GameBuilder::default().build_with_rooms(&[RoomType::Shop]);
            g.step_test(AscendStep::new(0, 0));
            found_rare = g
                .shop
                .cards
                .iter()
                .take(5)
                .any(|c| c.0.rarity() == CardRarity::Rare);
            if found_rare {
                break;
            }
        }
        assert!(found_rare);
    }

    #[test]
    fn test_invalid_shop_relic() {
        for _ in 0..10 {
            let g = GameBuilder::default().build_shop();
            for r in &g.shop.relics {
                assert_not_matches!(
                    r.0,
                    RelicClass::OldCoin
                        | RelicClass::TheCourier
                        | RelicClass::SmilingMask
                        | RelicClass::MawBank
                );
            }
        }
    }
}
