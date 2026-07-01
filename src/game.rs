use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use rand::RngExt;
use rand::seq::SliceRandom;

use crate::action::Action;
use crate::actions::block::BlockAction;
use crate::actions::damage::{DamageAction, DamageType};
use crate::actions::discard_card::DiscardCardAction;
use crate::actions::draw::DrawAction;
use crate::actions::exhaust_card::ExhaustCardAction;
use crate::actions::gain_energy::GainEnergyAction;
use crate::actions::gain_gold::GainGoldAction;
use crate::actions::gain_relic::GainRelicAction;
use crate::actions::gain_status::GainStatusAction;
use crate::actions::heal::HealAction;
use crate::actions::play_card::PlayCardAction;
use crate::actions::reduce_status::ReduceStatusAction;
use crate::actions::remove_status::RemoveStatusAction;
use crate::actions::use_potion::UsePotionAction;
use crate::blessings::{Blessing, ChooseBlessingGameState};
use crate::campfire::CampfireGameState;
use crate::card::{Card, CardPile, CardRef};
use crate::cards::{CardClass, CardCost, CardRarity, CardType};
use crate::chest::{ChestSize, ClosedChestGameState};
use crate::combat::RollEliteCombatGameState;
use crate::combat::{RollBossCombatGameState, RollCombatGameState};
use crate::creature::{Creature, CreatureState};
use crate::draw_pile::DrawPile;
use crate::event::RollQuestionRoomGameState;
use crate::events::Event;
use crate::map::{MAP_WIDTH, Map, RoomType};
#[cfg(test)]
use crate::monster::MonsterBehavior;
use crate::monster::{Monster, MonsterInfo};
use crate::monsters::Combat;
use crate::potion::Potion;
use crate::queue::ActionQueue;
use crate::relic::{
    Relic, RelicClass, RelicRarity, all_boss_relics, all_common_relics, all_rare_relics,
    all_shop_relics, all_uncommon_relics,
};
use crate::rewards::{BossRewardGameState, Rewards};
use crate::rng::rand_slice;
use crate::shop::{Shop, ShopGameState};
use crate::state::{GameState, GameStateManager, Steps};
use crate::status::Status;
use crate::step::Step;

pub use crate::rng::Rand;

#[derive(Copy, Clone, PartialEq, Eq)]
pub struct CreatureRef(usize);

impl CreatureRef {
    pub fn player() -> Self {
        Self(0)
    }
    pub fn monster(n: usize) -> Self {
        Self(n + 1)
    }
    pub fn is_player(&self) -> bool {
        self.0 == 0
    }
    pub fn monster_index(&self) -> usize {
        assert!(!self.is_player());
        self.0 - 1
    }
}

impl std::fmt::Debug for CreatureRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.is_player() {
            write!(f, "player")
        } else {
            write!(f, "monster {}", self.0 - 1)
        }
    }
}

#[derive(Debug)]
struct GameStartGameState;

impl GameState for GameStartGameState {
    fn run(&self, game: &mut Game) {
        game.state.push_state(AscendGameState);
        game.state.push_state(ChooseBlessingGameState {
            rewards: Blessing::roll(&mut game.rng),
        });
        game.state.push_state(EnterActGameState);
    }
}

#[derive(Debug)]
#[cfg(test)]
struct TestStartNoBlessingGameState;

#[cfg(test)]
impl GameState for TestStartNoBlessingGameState {
    fn run(&self, game: &mut Game) {
        game.state.push_state(AscendGameState);
        game.state.push_state(EnterActGameState);
    }
}

#[derive(Debug)]
#[cfg(test)]
struct TestCombatStartGameState;

#[cfg(test)]
impl GameState for TestCombatStartGameState {
    fn run(&self, game: &mut Game) {
        game.roll_noop_monsters = true;
        game.state.push_state(RollCombatGameState);
    }
}

fn unceasing_top_should_trigger(game: &Game) -> bool {
    game.in_combat != CombatType::None
        // DrawAction is a no-op once combat is finished; without this guard the
        // relic keeps requesting a draw that never lands, spinning forever with
        // an empty hand after the last monster dies.
        && !game.combat_finished()
        && game.hand.is_empty()
        && game.has_relic(RelicClass::UnceasingTop)
        && !game.player.has_status(Status::NoDraw)
        && (!game.draw_pile.is_empty() || !game.discard_pile.is_empty())
}

#[derive(Clone, Debug)]
pub struct RunActionsGameState;

impl GameState for RunActionsGameState {
    fn clone_box(&self) -> Box<dyn GameState> {
        Box::new(self.clone())
    }
    fn run(&self, game: &mut Game) {
        if !game.action_queue.is_empty()
            || !game.card_queue.is_empty()
            || !game.monster_turn_queue_active.is_empty()
            || unceasing_top_should_trigger(game)
        {
            game.state.push_state(RunActionsGameState);
        }
        if let Some(a) = game.action_queue.pop() {
            a.run(game);
        } else if !game.card_queue.is_empty() {
            let play = game.card_queue.remove(0);
            if game.combat_finished() {
                return;
            }
            if game.can_play_card(&play) {
                game.action_queue.push_bot(play);
            } else if !play.is_duplicated {
                if play.force_exhaust {
                    game.action_queue.push_bot(ExhaustCardAction(play.card));
                } else {
                    game.action_queue.push_bot(DiscardCardAction(play.card));
                }
            }
        } else if !game.monster_turn_queue_active.is_empty() {
            let monster = game.monster_turn_queue_active.remove(0);
            if !game.get_creature(monster).is_actionable() {
                return;
            }
            let mi = game.calculate_monster_info();
            game.monsters[monster.monster_index()].behavior.take_turn(
                monster,
                &mut game.action_queue,
                &mi,
            );
        } else if unceasing_top_should_trigger(game) {
            game.action_queue.push_bot(DrawAction(1));
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct AscendStep {
    x: usize,
    y: usize,
    use_wing_boots: bool,
}

impl AscendStep {
    pub fn new(x: usize, y: usize) -> Self {
        Self {
            x,
            y,
            use_wing_boots: false,
        }
    }
    pub fn wing_boots(x: usize, y: usize) -> Self {
        Self {
            x,
            y,
            use_wing_boots: true,
        }
    }
}

impl Step for AscendStep {
    fn should_pop_state(&self) -> bool {
        false
    }
    fn run(&self, game: &mut Game) {
        game.floor += 1;
        game.map_position = Some((self.x, self.y));
        game.cur_event = None;
        match game.map.nodes[self.x][self.y].ty.unwrap() {
            RoomType::Monster => game.state.push_state(RollCombatGameState),
            RoomType::Elite => game.state.push_state(RollEliteCombatGameState),
            RoomType::Event => game.state.push_state(RollQuestionRoomGameState),
            RoomType::Campfire => game.state.push_state(RollCampfireGameState),
            RoomType::Shop => game.state.push_state(RollShopGameState),
            RoomType::Treasure => game.state.push_state(RollTreasureGameState),
            RoomType::Boss => game.state.push_state(RollBossCombatGameState),
            RoomType::BossTreasure => game.state.push_state(BossRewardGameState),
        };
        if self.use_wing_boots {
            game.set_relic_value(
                RelicClass::WingBoots,
                game.get_relic_value(RelicClass::WingBoots).unwrap() - 1,
            );
        }
        if game.get_relic_value(RelicClass::MawBank) == Some(1) {
            game.action_queue.push_bot(GainGoldAction(12));
            game.state.push_state(RunActionsGameState);
        }
    }
    fn description(&self, _: &Game) -> String {
        format!("ascend to ({}, {})", self.x, self.y)
    }
}

#[derive(Clone, Debug)]
struct AscendGameState;

impl GameState for AscendGameState {
    fn clone_box(&self) -> Box<dyn GameState> {
        Box::new(self.clone())
    }
    fn valid_steps(&self, game: &Game) -> Option<Steps> {
        let mut steps = Steps::default();
        match game.map_position {
            Some(p) => {
                for e in &game.map.nodes[p.0][p.1].edges {
                    steps.push(AscendStep::new(*e, p.1 + 1));
                }
                if game
                    .get_relic_value(RelicClass::WingBoots)
                    .is_some_and(|v| v > 0)
                {
                    for x in 0..MAP_WIDTH {
                        if game.map.nodes[x][p.1 + 1].ty.is_some()
                            && !game.map.nodes[p.0][p.1].edges.contains(&x)
                        {
                            steps.push(AscendStep::wing_boots(x, p.1 + 1));
                        }
                    }
                }
            }
            None => {
                for x in 0..MAP_WIDTH {
                    if !game.map.nodes[x][0].edges.is_empty() {
                        steps.push(AscendStep::new(x, 0));
                    }
                }
            }
        }
        Some(steps)
    }
}

#[derive(Debug)]
pub struct EnterActGameState;

impl GameState for EnterActGameState {
    fn run(&self, game: &mut Game) {
        game.potion_chance = 40;
        game.num_combats_this_act = 0;
        game.last_elite = None;
        game.combat_history.clear();
        game.map_position = None;
        game.map = Map::generate(&mut game.rng);
        if game.is_in_act(1) {
            game.event_one_time_pool = vec![
                Event::AccursedBlackSmith,
                Event::FaceTrader,
                Event::Lab,
                Event::WeMeetAgain,
                Event::Duplicator,
                Event::WomanInBlue,
            ];
        } else {
            game.action_queue.push_bot(HealAction::player(
                ((game.player.max_hp - game.player.cur_hp) as f32 * 0.75) as i32,
            ));
            game.state.push_state(RunActionsGameState);
        }
        game.event_act_pool.clear();
        game.event_shrine_pool = vec![
            // Event::WheelOfChange,
            Event::Transmorgrifier,
            // Event::MatchAndKeep,
            Event::Purifier,
            // Event::GoldenShrine,
            Event::Upgrade,
        ];
        if game.is_in_act(1) {
            game.event_act_pool = vec![
                Event::BigFish,
                Event::WorldOfGoop,
                Event::Sssserpent,
                Event::ShiningLight,
                Event::LivingWall,
            ];
            game.easy_pool_combats = vec![
                Combat::Cultist,
                Combat::JawWorm,
                Combat::TwoLouses,
                Combat::SmallSlimes,
            ];
            game.add_hard_pool_combat(Combat::BlueSlaver, 4);
            game.add_hard_pool_combat(Combat::GremlinGang, 2);
            game.add_hard_pool_combat(Combat::Looter, 4);
            game.add_hard_pool_combat(Combat::LargeSlime, 4);
            game.add_hard_pool_combat(Combat::LotsOfSlimes, 2);
            game.add_hard_pool_combat(Combat::ExordiumThugs, 3);
            game.add_hard_pool_combat(Combat::ExordiumWildlife, 3);
            game.add_hard_pool_combat(Combat::RedSlaver, 2);
            game.add_hard_pool_combat(Combat::ThreeLouses, 4);
            game.add_hard_pool_combat(Combat::TwoFungiBeasts, 4);

            game.elites = vec![Combat::GremlinNob, Combat::Lagavulin, Combat::ThreeSentries];

            game.set_boss(&[Combat::Hexaghost, Combat::SlimeBoss, Combat::Guardian]);
        } else if game.is_in_act(2) {
            game.event_act_pool = vec![];
            game.easy_pool_combats = vec![
                Combat::SphericGuardian,
                Combat::Chosen,
                Combat::ShellParasite,
                Combat::ThreeByrds,
                Combat::TwoThieves,
            ];
            game.add_hard_pool_combat(Combat::ChosenAndByrd, 2);
            game.add_hard_pool_combat(Combat::ChosenAndCultist, 3);
            game.add_hard_pool_combat(Combat::SentryAndSphericGuardian, 2);
            game.add_hard_pool_combat(Combat::SnakePlant, 6);
            game.add_hard_pool_combat(Combat::Snecko, 4);
            game.add_hard_pool_combat(Combat::CenturionAndMystic, 6);
            game.add_hard_pool_combat(Combat::ThreeCultists, 3);
            game.add_hard_pool_combat(Combat::ShelledParasiteAndFungiBeast, 3);

            game.elites = vec![
                Combat::GremlinLeader,
                Combat::BookOfStabbing,
                Combat::Slavers,
            ];

            game.set_boss(&[Combat::Collector, Combat::Automaton, Combat::Champ]);
        } else if game.is_in_act(3) {
            game.event_act_pool = vec![];
            game.easy_pool_combats = vec![
                Combat::ThreeDarklings,
                Combat::OrbWalker,
                Combat::ThreeShapes,
            ];
            game.add_hard_pool_combat(Combat::FourShapes, 1);
            game.add_hard_pool_combat(Combat::Maw, 1);
            game.add_hard_pool_combat(Combat::SphereAndTwoShapes, 1);
            game.add_hard_pool_combat(Combat::ThreeDarklings, 1);
            game.add_hard_pool_combat(Combat::SpireGrowth, 1);
            game.add_hard_pool_combat(Combat::Transient, 1);
            game.add_hard_pool_combat(Combat::JawWormHorde, 1);
            game.add_hard_pool_combat(Combat::WrithingMass, 1);

            game.elites = vec![Combat::Nemesis, Combat::Reptomancer, Combat::GiantHead];

            game.set_boss(&[Combat::TimeEater, Combat::AwakenedOne, Combat::DonuDeca]);
        }
    }
}

#[derive(Debug)]
pub struct RollCampfireGameState;

impl GameState for RollCampfireGameState {
    fn run(&self, game: &mut Game) {
        game.state.push_state(CampfireGameState);
        if game.has_relic(RelicClass::AncientTeaSet) {
            game.set_relic_value(RelicClass::AncientTeaSet, 1);
        }
        if game.has_relic(RelicClass::EternalFeather) {
            game.action_queue
                .push_bot(HealAction::player((game.master_deck.len() / 5 * 3) as i32));
            game.state.push_state(RunActionsGameState);
        }
    }
}

#[derive(Debug)]
pub struct RollShopGameState;

impl GameState for RollShopGameState {
    fn run(&self, game: &mut Game) {
        game.cur_room = Some(RoomType::Shop);
        game.shop = Shop::new(game);
        game.state.push_state(ShopGameState);
        if game.has_relic(RelicClass::MealTicket) {
            game.action_queue.push_bot(HealAction {
                target: CreatureRef::player(),
                amount: 15,
            });
            game.state.push_state(RunActionsGameState);
        }
    }
}

#[derive(Debug)]
pub struct RollTreasureGameState;

impl GameState for RollTreasureGameState {
    fn run(&self, game: &mut Game) {
        game.cur_room = Some(RoomType::Treasure);
        let size = match game.rng.random_range(0..100) {
            0..50 => ChestSize::Small,
            50..83 => ChestSize::Medium,
            _ => ChestSize::Large,
        };
        game.chest_size = Some(size);
        game.state.push_state(ClosedChestGameState);
    }
}

#[allow(dead_code)]
#[derive(Debug)]
struct VictoryGameState;

impl GameState for VictoryGameState {
    fn run(&self, game: &mut Game) {
        game.status = GameStatus::Victory;
    }
}

#[derive(Debug)]
struct DefeatGameState;

impl GameState for DefeatGameState {
    fn run(&self, game: &mut Game) {
        game.status = GameStatus::Defeat;
    }
}

#[derive(Eq, PartialEq, Debug)]
pub struct UsePotionStep {
    pub potion_index: usize,
    pub target: Option<usize>,
}

impl Step for UsePotionStep {
    fn should_pop_state(&self) -> bool {
        false
    }

    fn run(&self, game: &mut Game) {
        let p = game.take_potion(self.potion_index);
        game.action_queue.push_bot(UsePotionAction {
            potion: p,
            target: self.target.map(CreatureRef::monster),
        });
        game.state.push_state(RunActionsGameState);
    }

    fn description(&self, game: &Game) -> String {
        let mut s = format!(
            "use potion {} ({:?})",
            self.potion_index,
            game.potions[self.potion_index].unwrap()
        );
        if let Some(t) = self.target {
            s += &format!(
                " on monster {} ({})",
                t,
                game.monster_str(CreatureRef::monster(t))
            );
        }
        s
    }
}

#[derive(Eq, PartialEq, Debug)]
pub struct DiscardPotionStep {
    pub potion_index: usize,
}

impl Step for DiscardPotionStep {
    fn should_pop_state(&self) -> bool {
        false
    }

    fn run(&self, game: &mut Game) {
        game.take_potion(self.potion_index);
    }

    fn description(&self, game: &Game) -> String {
        format!(
            "discard potion {} ({:?})",
            self.potion_index,
            game.potions[self.potion_index].unwrap()
        )
    }
}

#[derive(Debug, Clone)]
pub enum GameStatus {
    Defeat,
    Victory,
    Combat,
}

#[derive(Default)]
#[allow(unused)]
pub struct GameBuilder {
    master_deck: Vec<(CardClass, bool)>,
    force_monsters: Option<Vec<Monster>>,
    monster_statuses: HashMap<Status, i32>,
    player_statuses: HashMap<Status, i32>,
    relics: Vec<RelicClass>,
    player_hp: Option<i32>,
    rng: Rand,
}

impl GameBuilder {
    // Seed the run's RNG for reproducible builds/evals.
    pub fn seed(mut self, seed: u64) -> Self {
        self.rng = Rand::seed_from_u64(seed);
        self
    }
    pub fn ironclad_starting_deck(mut self) -> Self {
        for _ in 0..5 {
            self.master_deck.push((CardClass::Strike, false));
        }
        for _ in 0..4 {
            self.master_deck.push((CardClass::Defend, false));
        }
        self.master_deck.push((CardClass::Bash, false));
        self.master_deck.push((CardClass::AscendersBane, false));
        self
    }
    pub fn add_card(mut self, c: CardClass) -> Self {
        self.master_deck.push((c, false));
        self
    }
    pub fn add_card_upgraded(mut self, c: CardClass) -> Self {
        self.master_deck.push((c, true));
        self
    }
    #[cfg(test)]
    pub fn add_cards(mut self, c: CardClass, amount: i32) -> Self {
        for _ in 0..amount {
            self.master_deck.push((c, false));
        }
        self
    }
    #[cfg(test)]
    pub fn add_cards_upgraded(mut self, c: CardClass, amount: i32) -> Self {
        for _ in 0..amount {
            self.master_deck.push((c, true));
        }
        self
    }

    #[cfg(test)]
    pub fn add_monster_status(mut self, s: Status, amount: i32) -> Self {
        self.monster_statuses.insert(s, amount);
        self
    }

    #[cfg(test)]
    pub fn add_player_status(mut self, s: Status, amount: i32) -> Self {
        self.player_statuses.insert(s, amount);
        self
    }
    pub fn add_relic(mut self, relic: RelicClass) -> Self {
        self.relics.push(relic);
        self
    }
    #[cfg(test)]
    pub fn set_player_hp(mut self, amount: i32) -> Self {
        self.player_hp = Some(amount);
        self
    }
    #[cfg(test)]
    pub fn build_combat_with_monster<M: MonsterBehavior + 'static>(mut self, m: M) -> Game {
        self.force_monsters = Some(vec![Monster::new(m, &mut self.rng)]);
        self.build_combat()
    }
    #[cfg(test)]
    pub fn build_combat_with_monster_rng<M: MonsterBehavior + 'static, F: Fn(&mut Rand) -> M>(
        mut self,
        mf: F,
    ) -> Game {
        let m = mf(&mut self.rng);
        self.force_monsters = Some(vec![Monster::new(m, &mut self.rng)]);
        self.build_combat()
    }
    #[cfg(test)]
    pub fn build_combat_with_monsters<
        M1: MonsterBehavior + 'static,
        M2: MonsterBehavior + 'static,
    >(
        mut self,
        m1: M1,
        m2: M2,
    ) -> Game {
        self.force_monsters = Some(vec![
            Monster::new(m1, &mut self.rng),
            Monster::new(m2, &mut self.rng),
        ]);
        self.build_combat()
    }
    #[cfg(test)]
    pub fn build_combat_with_monsters_3<
        M1: MonsterBehavior + 'static,
        M2: MonsterBehavior + 'static,
        M3: MonsterBehavior + 'static,
    >(
        mut self,
        m1: M1,
        m2: M2,
        m3: M3,
    ) -> Game {
        self.force_monsters = Some(vec![
            Monster::new(m1, &mut self.rng),
            Monster::new(m2, &mut self.rng),
            Monster::new(m3, &mut self.rng),
        ]);
        self.build_combat()
    }
    #[cfg(test)]
    pub fn build_combat(self) -> Game {
        let monster_statuses = self.monster_statuses.clone();
        let mut g = self.build_with_game_state(TestCombatStartGameState);
        for m in &mut g.monsters {
            for (&k, &v) in &monster_statuses {
                m.creature.set_status(k, v);
            }
        }
        g
    }
    #[cfg(test)]
    pub fn build_campfire(self) -> Game {
        self.build_with_game_state(CampfireGameState)
    }
    #[cfg(test)]
    pub fn build_shop(self) -> Game {
        self.build_with_game_state(RollShopGameState)
    }
    #[cfg(test)]
    pub fn build_with_rooms(self, rooms: &[RoomType]) -> Game {
        let mut g = self.build_with_game_state(TestStartNoBlessingGameState);
        g.map = Map::straight_single_path(rooms);
        g
    }
    pub fn build(self) -> Game {
        self.build_with_game_state(GameStartGameState)
    }
    pub fn build_with_game_state<T: GameState + 'static>(self, start_state: T) -> Game {
        let mut g = Game::new(self.rng, &self.master_deck);
        g.force_monsters = self.force_monsters;
        for (&k, &v) in &self.player_statuses {
            g.player.set_status(k, v);
        }
        for r in self.relics {
            g.action_queue.push_bot(GainRelicAction(r));
        }
        g.state.push_state(RunActionsGameState);
        g.run();
        if let Some(hp) = self.player_hp {
            g.player.cur_hp = hp;
        }
        g.state.push_state(start_state);
        g.run();
        g
    }
}

macro_rules! trigger {
    ($func_name:ident, $name:ident) => {
        pub fn $func_name(&mut self) {
            for r in &mut self.relics {
                r.$name(&mut self.action_queue);
            }
        }
    };
}

macro_rules! trigger_card {
    ($func_name:ident, $name:ident) => {
        pub fn $func_name(&mut self, play: &PlayCardAction) {
            for r in &mut self.relics {
                r.$name(&mut self.action_queue, &mut self.card_queue, play);
            }
        }
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CombatType {
    None,
    Normal,
    Elite,
    Boss,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum RareCardBaseChance {
    Normal,
    Shop,
    Elite,
    Boss,
}

pub struct Game {
    pub rng: Rand,

    pub action_queue: ActionQueue,
    pub state: GameStateManager,
    pub status: GameStatus,
    pub is_running: bool,

    pub map: Map,
    pub cur_room: Option<RoomType>,
    pub cur_event: Option<Event>,
    pub floor: i32,
    pub map_position: Option<(usize, usize)>,
    pub player: Creature,
    pub has_ruby_key: bool,
    pub has_emerald_key: bool,
    pub has_sapphire_key: bool,
    pub relics: Vec<Relic>,
    pub potions: Vec<Option<Potion>>,
    pub gold: i32,
    pub draw_per_turn: i32,
    pub master_deck: CardPile,
    next_id: u32,
    pub force_monsters: Option<Vec<Monster>>,
    pub roll_noop_monsters: bool,
    pub override_event_queue: Vec<Event>,

    pub num_combats_this_act: i32,
    pub combat_history: Vec<Combat>,
    pub last_elite: Option<Combat>,
    pub elites: Vec<Combat>,
    pub easy_pool_combats: Vec<Combat>,
    pub hard_pool_combats: Vec<Combat>,
    pub boss: Option<Combat>,

    pub event_shrine_pool: Vec<Event>,
    pub event_one_time_pool: Vec<Event>,
    pub event_act_pool: Vec<Event>,
    pub event_monster_chance: i32,
    pub event_chest_chance: i32,
    pub event_shop_chance: i32,

    pub common_relic_pool: Vec<RelicClass>,
    pub uncommon_relic_pool: Vec<RelicClass>,
    pub rare_relic_pool: Vec<RelicClass>,
    pub shop_relic_pool: Vec<RelicClass>,
    pub boss_relic_pool: Vec<RelicClass>,

    pub rewards: Rewards,
    pub potion_chance: i32,
    pub rare_card_chance: i32,

    pub boss_rewards: Vec<RelicClass>,

    pub chest_size: Option<ChestSize>,

    pub shop: Shop,
    pub shop_remove_count: i32,

    pub in_combat: CombatType,
    pub turn: i32,
    pub monsters: Vec<Monster>,
    pub smoke_bombed: bool,
    pub energy: i32,
    pub draw_pile: DrawPile<CardRef>,
    pub hand: CardPile,
    pub discard_pile: CardPile,
    pub exhaust_pile: CardPile,
    pub cur_card: Option<CardRef>,
    pub card_queue: Vec<PlayCardAction>,
    pub monster_turn_queue_all: Vec<CreatureRef>,
    pub monster_turn_queue_active: Vec<CreatureRef>,
    pub should_add_extra_decay_status: bool,
    pub num_cards_played_this_turn: i32,
    pub num_times_took_damage: i32,
    pub chosen_cards: Vec<CardRef>,
}

impl Game {
    pub const MAX_HAND_SIZE: i32 = 10;

    fn new(mut rng: Rand, master_deck: &[(CardClass, bool)]) -> Self {
        let mut common_relic_pool = all_common_relics();
        let mut uncommon_relic_pool = all_uncommon_relics();
        let mut rare_relic_pool = all_rare_relics();
        let mut shop_relic_pool = all_shop_relics();
        let mut boss_relic_pool = all_boss_relics();
        common_relic_pool.shuffle(&mut rng);
        uncommon_relic_pool.shuffle(&mut rng);
        rare_relic_pool.shuffle(&mut rng);
        shop_relic_pool.shuffle(&mut rng);
        boss_relic_pool.shuffle(&mut rng);
        let mut g = Self {
            map: Default::default(),
            cur_room: Default::default(),
            cur_event: Default::default(),
            num_combats_this_act: 0,
            combat_history: Default::default(),
            last_elite: None,
            elites: Default::default(),
            easy_pool_combats: Default::default(),
            hard_pool_combats: Default::default(),
            boss: None,
            floor: 0,
            map_position: Default::default(),
            player: Creature::new("Ironclad", 80),
            relics: Default::default(),
            common_relic_pool,
            uncommon_relic_pool,
            rare_relic_pool,
            shop_relic_pool,
            boss_relic_pool,
            monsters: Default::default(),
            potions: vec![None; 2],
            gold: 0,
            master_deck: Default::default(),
            shop: Default::default(),
            shop_remove_count: 0,
            potion_chance: 0,
            rare_card_chance: 0,
            turn: 0,
            in_combat: CombatType::None,
            smoke_bombed: false,
            energy: 0,
            rewards: Default::default(),
            boss_rewards: Default::default(),
            chest_size: Default::default(),
            draw_per_turn: 5,
            draw_pile: Default::default(),
            hand: Default::default(),
            discard_pile: Default::default(),
            exhaust_pile: Default::default(),
            cur_card: None,
            action_queue: Default::default(),
            card_queue: Default::default(),
            monster_turn_queue_active: Default::default(),
            monster_turn_queue_all: Default::default(),
            should_add_extra_decay_status: false,
            num_cards_played_this_turn: 0,
            num_times_took_damage: 0,
            force_monsters: Default::default(),
            roll_noop_monsters: false,
            override_event_queue: Default::default(),
            rng,
            state: Default::default(),
            chosen_cards: Default::default(),
            next_id: 1,
            status: GameStatus::Combat,
            is_running: false,
            event_act_pool: Default::default(),
            event_one_time_pool: Default::default(),
            event_shrine_pool: Default::default(),
            has_emerald_key: false,
            has_ruby_key: false,
            has_sapphire_key: false,
            event_monster_chance: 10,
            event_shop_chance: 3,
            event_chest_chance: 2,
        };

        for (c, u) in master_deck {
            let card = if *u {
                g.new_card_upgraded(*c)
            } else {
                g.new_card(*c)
            };
            g.master_deck.push(card);
        }
        g.player.cur_hp = (g.player.cur_hp as f32 * 0.9) as i32;

        g
    }

    pub fn set_debug(&mut self) {
        self.action_queue.set_debug();
        self.state.set_debug();
    }

    fn new_card_id(&mut self, c: CardClass) -> u32 {
        if matches!(c, CardClass::RitualDagger | CardClass::Rampage) {
            let ret = self.next_id;
            self.next_id += 1;
            ret
        } else {
            0
        }
    }

    pub fn new_card(&mut self, class: CardClass) -> CardRef {
        let id = self.new_card_id(class);
        let mut c = Card {
            class,
            upgrade_count: 0,
            cost: class.base_cost(),
            is_bottled: false,
            exhaust: class.base_exhausts(),
            base_increase: 0,
            id,
        };
        if class == CardClass::BloodForBlood {
            let cost = c.get_base_cost();
            c.update_cost((cost - self.num_times_took_damage).max(0));
        }
        Rc::new(RefCell::new(c))
    }

    pub fn new_card_upgraded(&mut self, class: CardClass) -> CardRef {
        let c = self.new_card(class);
        c.borrow_mut().upgrade();
        c
    }

    pub fn clone_card_ref_same_id(&self, c: &CardRef) -> CardRef {
        Rc::new(RefCell::new(c.borrow().clone()))
    }

    pub fn clone_card_ref_new_id(&mut self, c: &CardRef) -> CardRef {
        let mut c = c.borrow().clone();
        c.id = self.new_card_id(c.class);
        Rc::new(RefCell::new(c))
    }

    pub fn clone_card_new_id(&mut self, c: &Card) -> CardRef {
        let mut c = c.clone();
        c.id = self.new_card_id(c.class);
        Rc::new(RefCell::new(c))
    }

    pub fn add_hard_pool_combat(&mut self, combat: Combat, weight: i32) {
        for _ in 0..weight {
            self.hard_pool_combats.push(combat);
        }
    }

    pub fn set_boss(&mut self, bosses: &[Combat]) {
        self.boss = Some(rand_slice(&mut self.rng, bosses));
    }

    pub fn roll_rarity(&mut self, ty: RareCardBaseChance) -> CardRarity {
        let (mut rare_base, uncommon_base) = match ty {
            RareCardBaseChance::Normal => (3, 37),
            RareCardBaseChance::Shop => (9, 37),
            RareCardBaseChance::Elite => (10, 40),
            RareCardBaseChance::Boss => return CardRarity::Rare,
        };
        if self.has_relic(RelicClass::NlothsGift) {
            rare_base *= 3;
        }
        rare_base -= 5;
        let roll = self.rng.random_range(0..100);
        if roll < rare_base + self.rare_card_chance {
            return CardRarity::Rare;
        }
        if roll < rare_base + self.rare_card_chance + uncommon_base {
            return CardRarity::Uncommon;
        }
        CardRarity::Common
    }

    pub fn get_creature(&self, r: CreatureRef) -> &Creature {
        match r.0 {
            0 => &self.player,
            r => &self.monsters[r - 1].creature,
        }
    }

    pub fn get_creature_mut(&mut self, r: CreatureRef) -> &mut Creature {
        match r.0 {
            0 => &mut self.player,
            r => &mut self.monsters[r - 1].creature,
        }
    }

    pub fn get_alive_monsters(&self) -> Vec<CreatureRef> {
        let mut alive = vec![];
        for (i, m) in self.monsters.iter().enumerate() {
            if !m.creature.is_actionable() {
                continue;
            }
            alive.push(CreatureRef::monster(i));
        }
        alive
    }

    pub fn get_actionable_monsters_in_order(&self) -> Vec<CreatureRef> {
        let mut actionable = vec![];
        for &c in &self.monster_turn_queue_all {
            if !self.get_creature(c).is_actionable() {
                continue;
            }
            actionable.push(c);
        }
        actionable
    }

    pub fn get_random_alive_monster(&mut self) -> CreatureRef {
        let alive = self.get_alive_monsters();
        rand_slice(&mut self.rng, &alive)
    }

    pub fn calculate_monster_info(&self) -> MonsterInfo {
        MonsterInfo {
            num_alive_monsters: self.get_alive_monsters().len(),
            player_hp: self.player.cur_hp,
        }
    }

    pub fn calculate_damage(
        &self,
        amount: i32,
        source_ref: CreatureRef,
        target_ref: CreatureRef,
    ) -> i32 {
        let mut amount_f = amount as f32;
        let source = self.get_creature(source_ref);
        let target = self.get_creature(target_ref);
        if let Some(s) = source.get_status(Status::Strength) {
            amount_f += s as f32;
        }
        if let Some(s) = source.get_status(Status::Vigor) {
            amount_f += s as f32;
        }
        if source.has_status(Status::Weak) {
            amount_f *= 0.75;
        }
        if source.has_status(Status::PenNib) {
            amount_f *= 2.0;
        }
        if target.has_status(Status::Vulnerable) {
            if target_ref.is_player() && self.has_relic(RelicClass::OddMushroom) {
                amount_f *= 1.25;
            } else if !target_ref.is_player() && self.has_relic(RelicClass::PaperPhrog) {
                amount_f *= 1.75;
            } else {
                amount_f *= 1.5;
            }
        }
        0.max(amount_f as i32)
    }

    pub fn damage(&mut self, target: CreatureRef, mut amount: i32, ty: DamageType) {
        assert!(self.get_creature(target).is_actionable());
        if let DamageType::Attack {
            source,
            on_fatal: _,
        } = ty
        {
            if !self.get_creature(source).is_actionable() {
                return;
            }
            // calculate_damage applies Strength etc. and clamps to >= 0, so a
            // negative pre-calc amount (e.g. Heavy Blade with negative Strength)
            // is fine here.
            amount = self.calculate_damage(amount, source, target);
            if let Some(a) = self
                .get_creature(target)
                .get_status(Status::Thorns)
                .map(|v| DamageAction::thorns_no_rupture(v, source))
            {
                self.action_queue.push_top(a);
            }
            if let Some(a) = self
                .get_creature(target)
                .get_status(Status::FlameBarrier)
                .map(|v| DamageAction::thorns_no_rupture(v, source))
            {
                self.action_queue.push_top(a);
            }
        }
        assert!(amount >= 0);
        let c = self.get_creature_mut(target);
        if !c.is_actionable() {
            return;
        }
        let was_bloodied = c.is_bloodied();
        let had_block = c.block != 0;
        if c.has_status(Status::Intangible) {
            amount = amount.min(1);
        }
        if ty != DamageType::HPLoss {
            if c.block >= amount {
                c.block -= amount;
                amount = 0;
            } else {
                amount -= c.block;
                c.block = 0;
            }
        }
        amount = amount.min(c.cur_hp);
        if amount != 0 && c.has_status(Status::Buffer) {
            amount = 0;
            self.action_queue.push_bot(ReduceStatusAction {
                status: Status::Buffer,
                amount: 1,
                target,
            });
        }
        if target.is_player()
            && (1..=5).contains(&amount)
            && matches!(ty, DamageType::Attack { .. })
            && self.has_relic(RelicClass::Torii)
        {
            amount = 1;
        }
        if target.is_player() && self.has_relic(RelicClass::TungstenRod) {
            amount = (amount - 1).max(0);
        }
        if !target.is_player() && amount > 0 && amount < 5 && self.has_relic(RelicClass::Boot) {
            amount = 5;
        }
        let c = self.get_creature_mut(target);
        c.last_damage_taken = amount;
        if amount != 0 {
            c.cur_hp -= amount;
            if target.is_player() {
                self.trigger_relics_on_lose_hp();
                self.num_times_took_damage += 1;
            }
            if let Some(amount) = self.get_creature(target).get_status(Status::Angry)
                && matches!(ty, DamageType::Attack { .. })
            {
                self.action_queue.push_top(GainStatusAction {
                    status: Status::Strength,
                    amount,
                    target,
                });
            }
            // attack damage never procs rupture
            // hp loss always procs rupture
            // thorns proc rupture if source is player
            if matches!(
                ty,
                DamageType::HPLoss
                    | DamageType::Thorns {
                        procs_rupture: true
                    }
            ) && let Some(v) = self.get_creature(target).get_status(Status::Rupture)
            {
                self.action_queue.push_bot(GainStatusAction {
                    status: Status::Strength,
                    amount: v,
                    target,
                });
            }
            if matches!(ty, DamageType::Attack { .. })
                && self.get_creature(target).has_status(Status::PlatedArmor)
            {
                self.action_queue.push_bot(ReduceStatusAction {
                    status: Status::PlatedArmor,
                    amount: 1,
                    target,
                });
            }
            if matches!(ty, DamageType::Attack { .. })
                && let Some(v) = self.get_creature(target).get_status(Status::CurlUp)
            {
                self.action_queue.push_bot(BlockAction::monster(target, v));
                // push_top so multiple attacks don't trigger this before removing the status
                self.action_queue.push_top(RemoveStatusAction {
                    status: Status::CurlUp,
                    target,
                });
            }

            let update_blood_for_blood_cost = |card: &CardRef| {
                let mut c = card.borrow_mut();
                if c.class == CardClass::BloodForBlood {
                    if let CardCost::Cost { base_cost, .. } = c.cost {
                        c.update_cost(0.max(base_cost - 1));
                    } else {
                        panic!();
                    }
                }
            };
            for c in &self.hand {
                update_blood_for_blood_cost(c);
            }
            for c in &self.discard_pile {
                update_blood_for_blood_cost(c);
            }
            for c in self.draw_pile.get_all() {
                update_blood_for_blood_cost(c);
            }
            {
                let c = self.get_creature_mut(target);
                if let Some(v) = c.get_status(Status::ModeShift) {
                    c.set_status(Status::ModeShift, v - amount);
                }
            }
            if !target.is_player() {
                let m = &mut self.monsters[target.monster_index()];
                m.behavior.on_take_damage(target, &mut m.creature);
            }
        }

        if self.get_creature(target).cur_hp <= 0 {
            if !target.is_player() {
                if let Some(v) = self.get_creature(target).get_status(Status::SporeCloud) {
                    self.action_queue.push_top(GainStatusAction {
                        status: Status::Vulnerable,
                        amount: v,
                        target: CreatureRef::player(),
                    });
                }
                if let Some(v) = self.get_creature(target).get_status(Status::StolenGold) {
                    self.rewards.add_stolen_gold(v);
                }
                if self.has_relic(RelicClass::GremlinHorn) {
                    self.action_queue.push_bot(GainEnergyAction(1));
                    self.action_queue.push_bot(DrawAction(1));
                }
                self.monster_turn_queue_all.retain(|c| *c != target);
            } else if !self.has_relic(RelicClass::MarkOfTheBloom) {
                if let Some(i) = self.potions.iter().position(|p| *p == Some(Potion::Fairy)) {
                    self.take_potion(i);
                    let percent = if self.has_relic(RelicClass::SacredBark) {
                        0.6
                    } else {
                        0.3
                    };
                    let amount = ((self.player.max_hp as f32 * percent) as i32).max(1);
                    self.player.heal(amount);
                } else if self.get_relic_value(RelicClass::LizardTail) == Some(1) {
                    self.set_relic_value(RelicClass::LizardTail, 0);
                    let amount = ((self.player.max_hp as f32 * 0.5) as i32).max(1);
                    self.player.heal(amount);
                }
            }
            {
                let c = self.get_creature_mut(target);
                if c.cur_hp <= 0 {
                    c.state = CreatureState::Dead;
                }
            }
        }

        if !was_bloodied
            && self.get_creature(target).is_bloodied()
            && target.is_player()
            && self.has_relic(RelicClass::RedSkull)
        {
            self.action_queue.push_bot(GainStatusAction {
                status: Status::Strength,
                amount: 3,
                target: CreatureRef::player(),
            });
        }
        if self.get_creature(target).block == 0
            && had_block
            && !target.is_player()
            && self.has_relic(RelicClass::HandDrill)
        {
            self.action_queue.push_bot(GainStatusAction {
                status: Status::Vulnerable,
                amount: 2,
                target,
            });
        }

        if !self.get_creature(target).is_actionable() && target.is_player() {
            self.state.push_state(DefeatGameState);
        }
    }

    fn run(&mut self) {
        assert!(!matches!(
            self.status,
            GameStatus::Defeat | GameStatus::Victory
        ));
        assert!(!self.state.is_empty());

        let mut guard: u64 = 0;
        while !matches!(self.status, GameStatus::Defeat | GameStatus::Victory)
            && let Some(state) = self.state.pop_state()
        {
            guard += 1;
            if guard > 2_000_000 {
                eprintln!(
                    "game.run() runaway: top={:?} card_queue={} in_combat={:?} turn={} hand={:?}",
                    self.state,
                    self.card_queue.len(),
                    self.in_combat,
                    self.turn,
                    self.hand.iter().map(|c| c.borrow().class).collect::<Vec<_>>(),
                );
                panic!("game.run() exceeded iteration guard (infinite action loop)");
            }
            state.run(self);
            if state.valid_steps(self).is_some() {
                self.state.push_boxed_state(state);
                break;
            }
        }
    }

    #[cfg(test)]
    pub fn add_card_to_master_deck(&mut self, class: CardClass) {
        use crate::actions::add_card_class_to_master_deck::AddCardClassToMasterDeckAction;

        self.run_action(AddCardClassToMasterDeckAction(class));
    }

    #[cfg(test)]
    pub fn step_test_no_check_valid<T: Step>(&mut self, step: T) {
        self.step_impl(Box::new(step))
    }

    #[cfg(test)]
    pub fn step_test<T: Step>(&mut self, step: T) {
        let step = Box::new(step) as Box<dyn Step>;
        let valid_steps = self.valid_steps();
        if !valid_steps.contains(&step) {
            dbg!(&step);
            dbg!(&valid_steps);
            panic!();
        }
        self.step_impl(step)
    }

    pub fn step(&mut self, step_index: usize) {
        let step = self.valid_steps().remove(step_index);
        self.step_impl(step);
    }

    fn step_impl(&mut self, step: Box<dyn Step>) {
        assert!(!self.is_running);
        self.is_running = true;

        if step.should_pop_state() {
            self.state.pop_state();
        }
        step.run(self);
        self.run();

        self.is_running = false;
    }

    pub fn can_play_card(&self, play: &PlayCardAction) -> bool {
        let c = play.card.borrow();
        let can_play_ty = match c.class.ty() {
            CardType::Attack => !self.player.has_status(Status::Entangled),
            CardType::Skill | CardType::Power => true,
            CardType::Status => {
                c.class == CardClass::Slimed || self.has_relic(RelicClass::MedicalKit)
            }
            CardType::Curse => self.has_relic(RelicClass::BlueCandle),
        };
        if !can_play_ty {
            return false;
        }
        let can_play_class = match c.class {
            CardClass::Clash => self
                .hand
                .iter()
                .all(|c| c.borrow().class.ty() == CardType::Attack),
            CardClass::SecretTechnique => self
                .draw_pile
                .get_all()
                .iter()
                .any(|c| c.borrow().class.ty() == CardType::Skill),
            CardClass::SecretWeapon => self
                .draw_pile
                .get_all()
                .iter()
                .any(|c| c.borrow().class.ty() == CardType::Attack),
            _ => true,
        };
        if !can_play_class {
            return false;
        }
        if self.num_cards_played_this_turn >= 6 && self.has_relic(RelicClass::VelvetChoker) {
            return false;
        }
        if self.num_cards_played_this_turn >= 3
            && self
                .hand
                .iter()
                .any(|c| c.borrow().class == CardClass::Normality)
        {
            return false;
        }
        play.free || self.energy >= play.cost
    }

    pub fn valid_steps(&self) -> Vec<Box<dyn Step>> {
        let mut steps = self.state.peek().valid_steps(self).unwrap();
        if self.cur_event != Some(Event::WeMeetAgain) {
            for (pi, p) in self.potions.iter().enumerate() {
                if let Some(p) = p
                    && p.can_use_outside_combat()
                {
                    steps.push(UsePotionStep {
                        potion_index: pi,
                        target: None,
                    });
                }
            }
            for (pi, p) in self.potions.iter().enumerate() {
                if p.is_some() {
                    steps.push(DiscardPotionStep { potion_index: pi });
                }
            }
        }
        steps.steps
    }

    pub fn assert_no_actions(&self) {
        assert!(self.action_queue.is_empty());
        assert!(self.card_queue.is_empty());
        assert!(self.monster_turn_queue_active.is_empty());
    }

    // Forks the game so a search agent can play moves forward without touching
    // the real game. Only valid at a decision point (the action/card/monster
    // queues must be drained). Every owned card `Rc` is deep-cloned so mutating
    // the fork (costs, upgrades, piles) never aliases back into the original.
    pub fn clone_for_search(&self) -> Game {
        self.assert_no_actions();
        let clone_card = |c: &CardRef| Rc::new(RefCell::new(c.borrow().clone()));
        let clone_pile = |p: &CardPile| -> CardPile { p.iter().map(&clone_card).collect() };
        Game {
            rng: self.rng.clone(),
            action_queue: ActionQueue::default(),
            state: self.state.clone_for_search(),
            status: self.status.clone(),
            is_running: self.is_running,
            map: self.map.clone(),
            cur_room: self.cur_room,
            cur_event: self.cur_event,
            floor: self.floor,
            map_position: self.map_position,
            player: self.player.clone(),
            has_ruby_key: self.has_ruby_key,
            has_emerald_key: self.has_emerald_key,
            has_sapphire_key: self.has_sapphire_key,
            relics: self.relics.clone(),
            potions: self.potions.clone(),
            gold: self.gold,
            draw_per_turn: self.draw_per_turn,
            // Combat never mutates master-deck card contents (combat works on
            // clones made at combat start), so a shallow Rc clone is safe here
            // and avoids deep-cloning the whole deck on every fork. Valid only
            // because forks are stepped no further than the end of combat.
            master_deck: self.master_deck.clone(),
            next_id: self.next_id,
            force_monsters: self
                .force_monsters
                .as_ref()
                .map(|ms| ms.iter().map(|m| m.clone_for_search()).collect()),
            roll_noop_monsters: self.roll_noop_monsters,
            override_event_queue: self.override_event_queue.clone(),
            num_combats_this_act: self.num_combats_this_act,
            combat_history: self.combat_history.clone(),
            last_elite: self.last_elite,
            elites: self.elites.clone(),
            easy_pool_combats: self.easy_pool_combats.clone(),
            hard_pool_combats: self.hard_pool_combats.clone(),
            boss: self.boss,
            event_shrine_pool: self.event_shrine_pool.clone(),
            event_one_time_pool: self.event_one_time_pool.clone(),
            event_act_pool: self.event_act_pool.clone(),
            event_monster_chance: self.event_monster_chance,
            event_chest_chance: self.event_chest_chance,
            event_shop_chance: self.event_shop_chance,
            common_relic_pool: self.common_relic_pool.clone(),
            uncommon_relic_pool: self.uncommon_relic_pool.clone(),
            rare_relic_pool: self.rare_relic_pool.clone(),
            shop_relic_pool: self.shop_relic_pool.clone(),
            boss_relic_pool: self.boss_relic_pool.clone(),
            rewards: self.rewards.clone(),
            potion_chance: self.potion_chance,
            rare_card_chance: self.rare_card_chance,
            boss_rewards: self.boss_rewards.clone(),
            chest_size: self.chest_size,
            shop: self.shop.clone(),
            shop_remove_count: self.shop_remove_count,
            in_combat: self.in_combat,
            turn: self.turn,
            monsters: self.monsters.iter().map(|m| m.clone_for_search()).collect(),
            smoke_bombed: self.smoke_bombed,
            energy: self.energy,
            draw_pile: self.draw_pile.clone_map(&clone_card),
            hand: clone_pile(&self.hand),
            discard_pile: clone_pile(&self.discard_pile),
            exhaust_pile: clone_pile(&self.exhaust_pile),
            cur_card: self.cur_card.as_ref().map(&clone_card),
            card_queue: vec![],
            monster_turn_queue_all: self.monster_turn_queue_all.clone(),
            monster_turn_queue_active: vec![],
            should_add_extra_decay_status: self.should_add_extra_decay_status,
            num_cards_played_this_turn: self.num_cards_played_this_turn,
            num_times_took_damage: self.num_times_took_damage,
            chosen_cards: self.chosen_cards.iter().map(&clone_card).collect(),
        }
    }

    pub fn run_all_actions(&mut self) {
        assert!(!self.is_running);
        self.is_running = true;

        self.state.push_state(RunActionsGameState);
        self.run();

        self.is_running = false;
    }

    pub fn run_action<A: Action + 'static>(&mut self, a: A) {
        self.assert_no_actions();
        self.action_queue.push_bot(a);
        self.run_all_actions();
    }

    #[cfg(test)]
    pub fn throw_potion(&mut self, potion: Potion, target: Option<CreatureRef>) {
        self.run_action(UsePotionAction { potion, target });
    }

    #[cfg(test)]
    fn play_card_impl(&mut self, card: CardRef, target: Option<CreatureRef>) {
        self.assert_no_actions();
        let action = PlayCardAction::new(card, target, self);
        assert!(self.can_play_card(&action));
        self.card_queue.push(action);
        self.run_all_actions();
    }

    #[cfg(test)]
    pub fn play_card(&mut self, class: CardClass, target: Option<CreatureRef>) {
        let card = self.new_card(class);
        self.play_card_impl(card, target);
    }

    #[cfg(test)]
    pub fn play_card_upgraded(&mut self, class: CardClass, target: Option<CreatureRef>) {
        let card = self.new_card_upgraded(class);
        self.play_card_impl(card, target);
    }

    #[cfg(test)]
    pub fn add_card_to_hand(&mut self, class: CardClass) {
        use crate::actions::place_card_in_hand::PlaceCardInHandAction;

        let card = self.new_card(class);
        self.run_action(PlaceCardInHandAction(card));
    }

    #[cfg(test)]
    pub fn add_cards_to_hand(&mut self, class: CardClass, amount: i32) {
        for _ in 0..amount {
            self.add_card_to_hand(class);
        }
    }

    #[cfg(test)]
    pub fn add_card_to_hand_upgraded(&mut self, class: CardClass) {
        use crate::actions::place_card_in_hand::PlaceCardInHandAction;

        let card = self.new_card_upgraded(class);
        self.run_action(PlaceCardInHandAction(card));
    }

    #[cfg(test)]
    pub fn add_card_to_draw_pile(&mut self, class: CardClass) {
        use crate::actions::place_card_on_top_of_draw::PlaceCardOnTopOfDrawAction;

        let card = self.new_card(class);
        self.run_action(PlaceCardOnTopOfDrawAction(card));
    }

    #[cfg(test)]
    pub fn add_card_to_draw_pile_upgraded(&mut self, class: CardClass) {
        use crate::actions::place_card_on_top_of_draw::PlaceCardOnTopOfDrawAction;

        let card = self.new_card_upgraded(class);
        self.run_action(PlaceCardOnTopOfDrawAction(card));
    }

    #[cfg(test)]
    pub fn add_cards_to_draw_pile(&mut self, class: CardClass, amount: i32) {
        for _ in 0..amount {
            self.add_card_to_draw_pile(class);
        }
    }

    #[cfg(test)]
    pub fn add_card_to_discard_pile(&mut self, class: CardClass) {
        let card = self.new_card(class);
        self.run_action(DiscardCardAction(card));
    }

    #[cfg(test)]
    pub fn add_cards_to_discard_pile(&mut self, class: CardClass, amount: i32) {
        for _ in 0..amount {
            self.add_card_to_discard_pile(class);
        }
    }

    #[cfg(test)]
    pub fn add_card_to_exhaust_pile(&mut self, class: CardClass) {
        use crate::actions::exhaust_card::ExhaustCardAction;

        let card = self.new_card(class);
        self.run_action(ExhaustCardAction(card));
    }

    #[cfg(test)]
    pub fn get_hand_card(&self, class: CardClass) -> &CardRef {
        let cards = self
            .hand
            .iter()
            .filter(|c| c.borrow().class == class)
            .collect::<Vec<_>>();
        assert_eq!(cards.len(), 1);
        cards[0]
    }

    pub fn has_removable_cards(&self) -> bool {
        self.master_deck
            .iter()
            .any(|c| c.borrow().can_remove_from_master_deck())
    }

    pub fn has_upgradable_cards(&self) -> bool {
        self.master_deck.iter().any(|c| c.borrow().can_upgrade())
    }

    pub fn clear_all_piles(&mut self) {
        self.assert_no_actions();
        self.hand.clear();
        self.discard_pile.clear();
        self.draw_pile.clear();
        self.exhaust_pile.clear();
    }

    pub fn hand_is_full(&self) -> bool {
        self.hand.len() as i32 == Game::MAX_HAND_SIZE
    }

    pub fn combat_finished(&self) -> bool {
        self.monsters.iter().all(|m| !m.creature.is_actionable()) || self.smoke_bombed
    }

    pub fn no_monsters_targetable(&self) -> bool {
        self.monsters.iter().all(|m| !m.creature.is_actionable())
    }

    pub fn monster_str(&self, c: CreatureRef) -> String {
        let mut i = self.monsters[c.monster_index()].behavior.get_intent();
        i.modify_damage(c, self);
        format!("{}, intent: {:?}", self.get_creature(c).str(), i)
    }

    pub fn heal(&mut self, cref: CreatureRef, mut amount: i32) {
        if amount == 0 {
            return;
        }
        if self.in_combat != CombatType::None && self.has_relic(RelicClass::MagicFlower) {
            amount = (amount as f32 * 1.5).round() as i32;
        }
        let c = self.get_creature_mut(cref);
        let was_bloodied = c.cur_hp <= c.max_hp / 2;
        c.heal(amount);
        // trigger player on heal relics
        let is_bloodied = c.cur_hp <= c.max_hp / 2;
        if was_bloodied && !is_bloodied && cref.is_player() && self.has_relic(RelicClass::RedSkull)
        {
            self.action_queue.push_bot(GainStatusAction {
                status: Status::Strength,
                amount: -3,
                target: CreatureRef::player(),
            });
        }
    }

    pub fn increase_max_hp(&mut self, amount: i32) {
        self.player.increase_max_hp(amount);
        self.heal(CreatureRef::player(), amount);
    }

    pub fn is_in_act(&self, act: i32) -> bool {
        match act {
            1 => (0..=17).contains(&self.floor),
            2 => (18..=34).contains(&self.floor),
            3 => (35..=51).contains(&self.floor),
            4 => self.floor >= 52,
            _ => panic!(),
        }
    }

    #[cfg(test)]
    pub fn add_potion(&mut self, potion: Potion) {
        use crate::actions::gain_potion::GainPotionAction;

        self.run_action(GainPotionAction(potion));
    }
    pub fn take_potion(&mut self, i: usize) -> Potion {
        let p = self.potions[i].unwrap();
        self.potions[i] = None;
        p
    }
    pub fn next_relic(&mut self, rarity: RelicRarity) -> RelicClass {
        loop {
            let pool = match rarity {
                RelicRarity::Common => &mut self.common_relic_pool,
                RelicRarity::Uncommon => &mut self.uncommon_relic_pool,
                RelicRarity::Rare => &mut self.rare_relic_pool,
                RelicRarity::Shop => &mut self.shop_relic_pool,
                RelicRarity::Boss => &mut self.boss_relic_pool,
                _ => panic!(),
            };
            let r = pool.pop().unwrap();
            if r.can_spawn(self) {
                return r;
            }
        }
    }
    pub fn next_relic_weighted(&mut self) -> RelicClass {
        // 50% common
        // 33% uncommon
        // 17% rare
        let rarity = match self.rng.random_range(0..100) {
            0..50 => RelicRarity::Common,
            50..83 => RelicRarity::Uncommon,
            _ => RelicRarity::Rare,
        };
        self.next_relic(rarity)
    }
    pub fn next_relic_weighted_screenless(&mut self) -> RelicClass {
        loop {
            let r = self.next_relic_weighted();
            if !matches!(
                r,
                RelicClass::BottledFlame
                    | RelicClass::BottledLightning
                    | RelicClass::BottledTornado
                    | RelicClass::Whetstone
            ) {
                return r;
            }
        }
    }

    #[cfg(test)]
    pub fn add_relic(&mut self, class: RelicClass) {
        self.run_action(GainRelicAction(class));
    }
    #[cfg(test)]
    pub fn remove_relic(&mut self, class: RelicClass) {
        use crate::actions::remove_relic::RemoveRelicAction;

        self.run_action(RemoveRelicAction(class));
    }
    pub fn has_relic(&self, class: RelicClass) -> bool {
        self.relics.iter().any(|r| r.get_class() == class)
    }
    pub fn get_relic_value(&self, class: RelicClass) -> Option<i32> {
        self.relics
            .iter()
            .find(|r| r.get_class() == class)
            .map(|r| r.get_value())
    }
    pub fn set_relic_value(&mut self, class: RelicClass, v: i32) {
        self.relics
            .iter_mut()
            .find(|r| r.get_class() == class)
            .unwrap()
            .set_value(v);
    }
    trigger!(trigger_relics_on_shuffle, on_shuffle);
    trigger!(trigger_relics_at_pre_combat, at_pre_combat);
    trigger!(
        trigger_relics_at_combat_begin_pre_draw,
        at_combat_begin_pre_draw
    );
    trigger!(
        trigger_relics_at_combat_begin_post_draw,
        at_combat_begin_post_draw
    );
    trigger!(
        trigger_relics_at_turn_begin_pre_draw,
        at_turn_begin_pre_draw
    );
    trigger!(
        trigger_relics_at_turn_begin_post_draw,
        at_turn_begin_post_draw
    );
    trigger!(trigger_relics_at_turn_end, at_turn_end);
    trigger!(trigger_relics_on_lose_hp, on_lose_hp);
    trigger!(trigger_relics_at_combat_finish, at_combat_finish);
    trigger_card!(trigger_relics_on_card_played, on_card_played);
}

#[cfg(test)]
mod tests {
    use crate::{
        campfire::{CampfireRestStep, CampfireUpgradeStep},
        cards::CardClass,
        combat::PlayCardStep,
        events::Event,
        game::{AscendStep, GameBuilder},
        map::{MAP_WIDTH, Map, RoomType},
        master_deck::ChooseUpgradeMasterStep,
        rewards::{BossRewardSkipStep, RewardExitStep},
        state::ContinueStep,
    };

    #[test]
    fn test_game() {
        let mut g = GameBuilder::default()
            .add_card(CardClass::DebugKill)
            .build();
        g.roll_noop_monsters = true;
        g.map = Map::straight_single_path(&[
            RoomType::Monster,
            RoomType::Campfire,
            RoomType::Monster,
            RoomType::Campfire,
            RoomType::Event,
        ]);

        g.step(1);
        g.step_test(AscendStep::new(0, 0));
        g.step_test(PlayCardStep {
            hand_index: 0,
            target: Some(0),
        });
        g.step_test(RewardExitStep);
        g.step_test(AscendStep::new(0, 1));
        g.step_test(CampfireRestStep);
        g.step_test(AscendStep::new(0, 2));
        g.step_test(PlayCardStep {
            hand_index: 0,
            target: Some(0),
        });
        g.step_test(RewardExitStep);
        g.step_test(AscendStep::new(0, 3));
        g.step_test(CampfireUpgradeStep);
        g.step_test(ChooseUpgradeMasterStep { master_index: 0 });

        g.override_event_queue.push(Event::AccursedBlackSmith);
        g.step_test(AscendStep::new(0, 4));
        g.step_test(ContinueStep);
    }

    #[test]
    fn test_enter_act_2() {
        let mut g = GameBuilder::default().build_with_rooms(&[RoomType::BossTreasure]);
        g.floor = 16;
        g.step_test(AscendStep::new(0, 0));
        g.step_test(BossRewardSkipStep);
        let count_nodes = (0..MAP_WIDTH)
            .filter(|x| !g.map.nodes[*x][0].edges.is_empty())
            .count();
        assert!(count_nodes > 1);
        assert_eq!(count_nodes, g.valid_steps().len());
        assert_eq!(g.floor, 17);
        g.step(0);
        assert_eq!(g.floor, 18);
    }
}
