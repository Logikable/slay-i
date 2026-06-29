use lazy_static::lazy_static;

use crate::{
    actions::{
        add_card_class_to_master_deck::AddCardClassToMasterDeckAction,
        block::BlockAction,
        choose_card_to_shuffle_into_draw::ChooseCardToShuffleIntoDrawAction,
        choose_discovery::{ChooseDiscoveryAction, ChooseDiscoveryType},
        choose_gamble::ChooseGambleAction,
        damage::DamageAction,
        damage_all_monsters::DamageAllMonstersAction,
        discount_random_card_in_hand::DiscountRandomCardInHandAction,
        draw::DrawAction,
        duvu::DuvuAction,
        enchiridion::EnchiridionAction,
        gain_energy::GainEnergyAction,
        gain_gold::GainGoldAction,
        gain_status::GainStatusAction,
        gain_status_all_monsters::GainStatusAllMonstersAction,
        heal::HealAction,
        increase_draw_per_turn::IncreaseDrawPerTurnAction,
        increase_max_hp::IncreaseMaxHPAction,
        increase_potion_slots::IncreasePotionSlotsAction,
        meat_on_the_bone::MeatOnTheBoneAction,
        orichalcum::OrichalcumAction,
        pantograph::PantographAction,
        play_card::PlayCardAction,
        preserved_insect::PreservedInsectAction,
        red_skull::RedSkullAction,
        remove_all_debuffs::RemoveAllDebuffsAction,
        set_hp_all_monsters::SetHPAllMonstersAction,
        shuffle_card_into_draw::ShuffleCardIntoDrawAction,
        sling_of_courage::SlingOfCourageAction,
        try_remove_card_from_master_deck::TryRemoveCardFromMasterDeckAction,
        upgrade_random_in_hand::UpgradeRandomInHandAction,
        upgrade_random_in_master::{UpgradeRandomInMasterAction, UpgradeTwoRandomInMasterAction},
    },
    cards::{CardClass, CardType},
    game::{CreatureRef, Game, RareCardBaseChance, RunActionsGameState},
    master_deck::{
        ChooseBottledCardGameState, ChooseDuplicateCardInMasterGameState,
        ChooseRemoveFromMasterGameState, ChooseTransformMasterGameState,
        TransformChosenCardsGameState,
    },
    potion::random_potion_weighted,
    queue::ActionQueue,
    rewards::{Rewards, RewardsGameState},
    state::{GameState, GameStateManager},
    status::Status,
};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum RelicRarity {
    Starter,
    Common,
    Uncommon,
    Rare,
    Shop,
    Event,
    Boss,
}

macro_rules! r {
    ($($name:ident => $rarity:expr),+,) => {
        #[derive(Clone, Copy, PartialEq, Eq, Debug)]
        pub enum RelicClass {
            $(
                $name,
            )+
        }
        impl RelicClass {
            pub fn rarity(&self) -> RelicRarity {
                use RelicRarity::*;
                match self {
                    $(Self::$name => $rarity,)+
                }
            }
        }
        impl RelicClass {
            pub fn all() -> Vec<Self> {
                vec![$(Self::$name,)+]
            }
        }
    };
}

r!(
    BurningBlood => Starter,

    Akabeko => Common,
    Anchor => Common,
    AncientTeaSet => Common,
    ArtOfWar => Common,
    BagOfMarbles => Common,
    BagOfPrep => Common,
    BloodVial => Common,
    BronzeScales => Common,
    CentennialPuzzle => Common,
    CeramicFish => Common,
    DreamCatcher => Common,
    HappyFlower => Common,
    JuzuBracelet => Common,
    Lantern => Common,
    MawBank => Common,
    MealTicket => Common,
    Nunchaku => Common,
    OddlySmoothStone => Common,
    Omamori => Common,
    Orichalcum => Common,
    PenNib => Common,
    PotionBelt => Common,
    PreservedInsect => Common,
    RegalPillow => Common,
    SmilingMask => Common,
    Strawberry => Common,
    Boot => Common,
    TinyChest => Common,
    ToyOrnithopter => Common,
    Vajra => Common,
    WarPaint => Common,
    Whetstone => Common,
    RedSkull => Common,

    BlueCandle => Uncommon,
    BottledFlame => Uncommon,
    BottledLightning => Uncommon,
    BottledTornado => Uncommon,
    DarkstonePeriapt => Uncommon,
    EternalFeather => Uncommon,
    FrozenEgg => Uncommon,
    GremlinHorn => Uncommon,
    HornCleat => Uncommon,
    InkBottle => Uncommon,
    Kunai => Uncommon,
    LetterOpener => Uncommon,
    Matryoshka => Uncommon,
    MeatOnTheBone => Uncommon,
    MercuryHourglass => Uncommon,
    MoltenEgg => Uncommon,
    MummifiedHand => Uncommon,
    OrnamentalFan => Uncommon,
    Pantograph => Uncommon,
    Pear => Uncommon,
    QuestionCard => Uncommon,
    Shruiken => Uncommon,
    SingingBowl => Uncommon,
    StrikeDummy => Uncommon,
    Sundial => Uncommon,
    TheCourier => Uncommon,
    ToxicEgg => Uncommon,
    WhiteBeastStatue => Uncommon,
    PaperPhrog => Uncommon,
    SelfFormingClay => Uncommon,

    BirdFacedUrn => Rare,
    Calipers => Rare,
    CaptainsWheel => Rare,
    DeadBranch => Rare,
    DuVuDoll => Rare,
    FossilizedHelix => Rare,
    GamblingChip => Rare,
    Ginger => Rare,
    Girya => Rare,
    IceCream => Rare,
    IncenseBurner => Rare,
    LizardTail => Rare,
    Mango => Rare,
    OldCoin => Rare,
    PeacePipe => Rare,
    Pocketwatch => Rare,
    PrayerWheel => Rare,
    Shovel => Rare,
    StoneCalendar => Rare,
    ThreadAndNeedle => Rare,
    Torii => Rare,
    TungstenRod => Rare,
    Turnip => Rare,
    UnceasingTop => Rare,
    WingBoots => Rare,
    ChampionBelt => Rare,
    CharonsAshes => Rare,
    MagicFlower => Rare,

    Cauldron => Shop,
    ChemicalX => Shop,
    ClockworkSouvenir => Shop,
    DollysMirror => Shop,
    FrozenEye => Shop,
    HandDrill => Shop,
    LeesWaffle => Shop,
    MedicalKit => Shop,
    MembershipCard => Shop,
    OrangePellets => Shop,
    Orrery => Shop,
    // PrismaticShard => Shop, // not supported
    SlingOfCourage => Shop,
    StrangeSpoon => Shop,
    TheAbacus => Shop,
    Toolbox => Shop,
    Brimstone => Shop,

    Astrolabe => Boss,
    BlackStar => Boss,
    BustedCrown => Boss,
    CallingBell => Boss,
    CoffeeDripper => Boss,
    CursedKey => Boss,
    Ectoplasm => Boss,
    EmptyCage => Boss,
    FusionHammer => Boss,
    PandorasBox => Boss,
    PhilosophersStone => Boss, // TODO
    // RunicDome => Boss, // not supported
    RunicPyramid => Boss,
    SacredBark => Boss,
    SlaversCollar => Boss,
    SneckoEye => Boss,
    Sozu => Boss,
    TinyHouse => Boss,
    VelvetChoker => Boss,
    BlackBlood => Boss,
    MarkOfPain => Boss,
    RunicCube => Boss,

    BloodyIdol => Event,
    CultistHeadpiece => Event,
    Enchiridion => Event,
    FaceOfCleric => Event,
    GoldenIdol => Event,
    GremlinVisage => Event,
    MarkOfTheBloom => Event,
    MutagenicStrength => Event,
    NlothsGift => Event,
    NlothsHungryFace => Event,
    Necronomicon => Event,
    NeowsLament => Event,
    NilrysCodex => Event,
    OddMushroom => Event,
    RedMask => Event,
    SpiritPoop => Event,
    SsserpentHead => Event,
    WarpedTongs => Event,
);

impl RelicClass {
    pub fn is_campfire_relic(&self) -> bool {
        use RelicClass::*;
        matches!(self, Girya | PeacePipe | Shovel)
    }

    pub fn can_spawn(&self, game: &Game) -> bool {
        if self.is_campfire_relic()
            && game
                .relics
                .iter()
                .filter(|r| r.get_class().is_campfire_relic())
                .count()
                >= 2
        {
            return false;
        }
        use RelicClass::*;
        match self {
            BlackBlood => game.has_relic(BurningBlood),
            Ectoplasm => game.floor <= 20,
            TinyChest => game.floor <= 35,
            WingBoots | Matryoshka => game.floor <= 40,
            TheCourier | PotionBelt | ToxicEgg | FrozenEgg | MoltenEgg | MealTicket
            | SingingBowl | DreamCatcher | MawBank | OldCoin | Shovel | Girya | PeacePipe
            | CeramicFish | Omamori | PrayerWheel | MeatOnTheBone | DarkstonePeriapt
            | AncientTeaSet | RegalPillow => game.floor <= 48,
            PreservedInsect => game.floor <= 52,
            BottledFlame => game
                .master_deck
                .iter()
                .any(|c| c.borrow().class.ty() == CardType::Attack),
            BottledLightning => game
                .master_deck
                .iter()
                .any(|c| c.borrow().class.ty() == CardType::Skill),
            BottledTornado => game
                .master_deck
                .iter()
                .any(|c| c.borrow().class.ty() == CardType::Power),
            _ => true,
        }
    }
}

type RelicCallback = fn(&mut i32, &mut ActionQueue);
type RelicEquipCallback = fn(&mut i32, &mut ActionQueue, &mut GameStateManager);
type RelicCardCallback = fn(&mut i32, &mut ActionQueue, &mut Vec<PlayCardAction>, &PlayCardAction);

impl RelicClass {
    pub fn on_equip(&self) -> Option<RelicEquipCallback> {
        use RelicClass::*;
        match self {
            LizardTail | MawBank | NlothsHungryFace => Some(equip_set_1),
            Omamori | Matryoshka => Some(equip_set_2),
            NeowsLament | WingBoots => Some(equip_set_3),
            WarPaint => Some(war_paint),
            Whetstone => Some(whetstone),
            SneckoEye => Some(snecko_eye_equip),
            OldCoin => Some(old_coin),
            PotionBelt => Some(potion_belt),
            LeesWaffle => Some(lees_waffle),
            Mango => Some(mango),
            Pear => Some(pear),
            Strawberry => Some(strawberry),
            Necronomicon => Some(necronomicon_equip),
            DollysMirror => Some(dollys_mirror),
            Orrery => Some(orrery),
            Cauldron => Some(cauldron),
            EmptyCage => Some(empty_cage),
            CallingBell => Some(calling_bell),
            PandorasBox => Some(pandoras_box),
            Astrolabe => Some(astrolabe),
            BottledFlame => Some(bottled_flame),
            BottledLightning => Some(bottled_lightning),
            BottledTornado => Some(bottled_tornado),
            TinyHouse => Some(tiny_house),
            _ => None,
        }
    }
    pub fn on_unequip(&self) -> Option<RelicCallback> {
        use RelicClass::*;
        match self {
            SneckoEye => Some(snecko_eye_unequip),
            Necronomicon => Some(necronomicon_unequip),
            _ => None,
        }
    }
    pub fn on_shuffle(&self) -> Option<RelicCallback> {
        use RelicClass::*;
        match self {
            Sundial => Some(sundial),
            TheAbacus => Some(abacus),
            _ => None,
        }
    }
    pub fn at_pre_combat(&self) -> Option<RelicCallback> {
        use RelicClass::*;
        match self {
            HornCleat | CaptainsWheel | ArtOfWar | StoneCalendar => Some(set_value_0),
            CentennialPuzzle => Some(set_value_1),
            Pocketwatch => Some(set_value_99),
            SneckoEye => Some(snecko_eye_confused),
            Enchiridion => Some(enchiridion),
            _ => None,
        }
    }
    pub fn at_combat_finish(&self) -> Option<RelicCallback> {
        use RelicClass::*;
        match self {
            MeatOnTheBone => Some(meat_on_the_bone),
            BurningBlood => Some(burning_blood),
            BlackBlood => Some(black_blood),
            FaceOfCleric => Some(face_of_cleric),
            _ => None,
        }
    }
    pub fn at_combat_begin_pre_draw(&self) -> Option<RelicCallback> {
        use RelicClass::*;
        match self {
            NeowsLament => Some(neows_lament),
            BloodVial => Some(blood_vial),
            AncientTeaSet => Some(ancient_tea_set),
            Lantern => Some(lantern),
            GremlinVisage => Some(gremlin_visage),
            MutagenicStrength => Some(mutagenic_strength),
            ClockworkSouvenir => Some(clockwork_souvenir),
            RedMask => Some(red_mask),
            BagOfMarbles => Some(bag_of_marbles),
            BronzeScales => Some(bronze_scales),
            Vajra => Some(vajra),
            Girya => Some(girya),
            OddlySmoothStone => Some(oddly_smooth_stone),
            PreservedInsect => Some(preserved_insect),
            SlingOfCourage => Some(sling_of_courage),
            DuVuDoll => Some(du_vu_doll),
            Akabeko => Some(akabeko),
            PenNib => Some(pen_nib_start),
            FossilizedHelix => Some(fossilized_helix),
            ThreadAndNeedle => Some(thread_and_needle),
            RedSkull => Some(red_skull),
            Toolbox => Some(toolbox),
            Pantograph => Some(pantograph),
            _ => None,
        }
    }
    pub fn at_combat_begin_post_draw(&self) -> Option<RelicCallback> {
        use RelicClass::*;
        match self {
            MarkOfPain => Some(mark_of_pain),
            BagOfPrep => Some(bag_of_prep),
            Anchor => Some(anchor),
            _ => None,
        }
    }
    pub fn on_card_played(&self) -> Option<RelicCardCallback> {
        use RelicClass::*;
        match self {
            BlueCandle => Some(blue_candle),
            Kunai => Some(kunai),
            Shruiken => Some(shruiken),
            InkBottle => Some(ink_bottle),
            LetterOpener => Some(letter_opener),
            OrnamentalFan => Some(ornamental_fan),
            Nunchaku => Some(nunchaku),
            BirdFacedUrn => Some(bird_faced_urn),
            ArtOfWar => Some(art_of_war_card_played),
            Pocketwatch => Some(pocketwatch_card_played),
            PenNib => Some(pen_nib),
            Necronomicon => Some(necronomicon),
            MummifiedHand => Some(mummified_hand),
            OrangePellets => Some(orange_pellets),
            _ => None,
        }
    }
    pub fn at_turn_begin_pre_draw(&self) -> Option<RelicCallback> {
        use RelicClass::*;
        match self {
            Kunai | Shruiken | LetterOpener | OrnamentalFan | OrangePellets => Some(set_value_0),
            Necronomicon => Some(set_value_1),
            HornCleat => Some(horn_cleat),
            CaptainsWheel => Some(captains_wheel),
            HappyFlower => Some(happy_flower),
            ArtOfWar => Some(art_of_war),
            IncenseBurner => Some(incense_burner),
            MercuryHourglass => Some(mercury_hourglass),
            Brimstone => Some(brimstone),
            _ => None,
        }
    }
    pub fn at_turn_begin_post_draw(&self) -> Option<RelicCallback> {
        use RelicClass::*;
        match self {
            Pocketwatch => Some(pocketwatch),
            WarpedTongs => Some(warped_tongs),
            GamblingChip => Some(gambling_chip),
            _ => None,
        }
    }
    pub fn on_lose_hp(&self) -> Option<RelicCallback> {
        use RelicClass::*;
        match self {
            CentennialPuzzle => Some(centennial_puzzle),
            SelfFormingClay => Some(self_forming_clay),
            RunicCube => Some(runic_cube),
            _ => None,
        }
    }
    pub fn at_turn_end(&self) -> Option<RelicCallback> {
        use RelicClass::*;
        match self {
            StoneCalendar => Some(stone_calendar),
            Orichalcum => Some(orichalcum),
            NilrysCodex => Some(nilrys_codex),
            _ => None,
        }
    }
}

fn equip_set_1(v: &mut i32, _: &mut ActionQueue, _: &mut GameStateManager) {
    *v = 1;
}

fn equip_set_2(v: &mut i32, _: &mut ActionQueue, _: &mut GameStateManager) {
    *v = 2;
}

fn equip_set_3(v: &mut i32, _: &mut ActionQueue, _: &mut GameStateManager) {
    *v = 3;
}

fn set_value_0(v: &mut i32, _: &mut ActionQueue) {
    *v = 0;
}

fn set_value_1(v: &mut i32, _: &mut ActionQueue) {
    *v = 1;
}

fn set_value_99(v: &mut i32, _: &mut ActionQueue) {
    *v = 99;
}

fn inc_wrap(v: &mut i32, max: i32) -> bool {
    *v += 1;
    if *v == max {
        *v = 0;
        true
    } else {
        false
    }
}

fn pocketwatch_card_played(
    v: &mut i32,
    _: &mut ActionQueue,
    _: &mut Vec<PlayCardAction>,
    _: &PlayCardAction,
) {
    *v += 1;
}

fn pocketwatch(v: &mut i32, queue: &mut ActionQueue) {
    if *v <= 3 {
        queue.push_bot(DrawAction(3));
    }
    *v = 0;
}

fn warped_tongs(_: &mut i32, queue: &mut ActionQueue) {
    queue.push_bot(UpgradeRandomInHandAction());
}

fn gambling_chip(_: &mut i32, queue: &mut ActionQueue) {
    queue.push_bot(ChooseGambleAction());
}

fn centennial_puzzle(v: &mut i32, queue: &mut ActionQueue) {
    if *v == 1 {
        *v = 0;
        // push_top is intentional
        queue.push_top(DrawAction(3));
    }
}

fn self_forming_clay(_: &mut i32, queue: &mut ActionQueue) {
    // push_top is intentional
    queue.push_top(GainStatusAction {
        status: Status::NextTurnBlock,
        amount: 3,
        target: CreatureRef::player(),
    });
}

fn runic_cube(_: &mut i32, queue: &mut ActionQueue) {
    // push_top is intentional
    queue.push_top(DrawAction(1));
}

fn pen_nib_start(v: &mut i32, queue: &mut ActionQueue) {
    if *v == 9 {
        queue.push_bot(GainStatusAction {
            status: Status::PenNib,
            amount: 1,
            target: CreatureRef::player(),
        });
    }
}

fn necronomicon_equip(_: &mut i32, queue: &mut ActionQueue, _: &mut GameStateManager) {
    queue.push_bot(AddCardClassToMasterDeckAction(CardClass::Necronomicurse));
}

fn dollys_mirror(_: &mut i32, _: &mut ActionQueue, state: &mut GameStateManager) {
    state.push_state(ChooseDuplicateCardInMasterGameState);
}

#[derive(Debug)]
pub struct OrreryGameState;

impl GameState for OrreryGameState {
    fn run(&self, game: &mut Game) {
        for _ in 0..5 {
            let cards = Rewards::gen_card_reward(game, RareCardBaseChance::Shop);
            game.rewards.add_cards(cards);
        }
        game.state.push_state(RewardsGameState);
    }
}

fn orrery(_: &mut i32, _: &mut ActionQueue, state: &mut GameStateManager) {
    state.push_state(OrreryGameState);
}

#[derive(Debug)]
pub struct CauldronGameState;

impl GameState for CauldronGameState {
    fn run(&self, game: &mut Game) {
        // the game creates and throws away a card reward
        let _ = Rewards::gen_card_reward(game, RareCardBaseChance::Shop);
        for _ in 0..5 {
            game.rewards
                .add_potion(random_potion_weighted(&mut game.rng));
        }
        game.state.push_state(RewardsGameState);
    }
}

fn cauldron(_: &mut i32, _: &mut ActionQueue, state: &mut GameStateManager) {
    state.push_state(CauldronGameState);
}

#[derive(Debug)]
pub struct CallingBellGameState;

impl GameState for CallingBellGameState {
    fn run(&self, game: &mut Game) {
        let r = game.next_relic(RelicRarity::Common);
        game.rewards.add_relic(r);
        let r = game.next_relic(RelicRarity::Uncommon);
        game.rewards.add_relic(r);
        let r = game.next_relic(RelicRarity::Rare);
        game.rewards.add_relic(r);
        game.state.push_state(RewardsGameState);

        game.action_queue
            .push_bot(AddCardClassToMasterDeckAction(CardClass::CurseOfTheBell));
        game.state.push_state(RunActionsGameState);
    }
}

fn calling_bell(_: &mut i32, _: &mut ActionQueue, state: &mut GameStateManager) {
    state.push_state(CallingBellGameState);
}

#[derive(Debug)]
pub struct PandorasBoxGameState;

impl GameState for PandorasBoxGameState {
    fn run(&self, game: &mut Game) {
        for i in (0..game.master_deck.len()).rev() {
            if matches!(
                game.master_deck[i].borrow().class,
                CardClass::Strike | CardClass::Defend
            ) {
                let c = game.master_deck.remove(i);
                game.chosen_cards.push(c);
            }
        }
        game.state
            .push_state(TransformChosenCardsGameState { upgrade: false });
    }
}

fn pandoras_box(_: &mut i32, _: &mut ActionQueue, state: &mut GameStateManager) {
    state.push_state(PandorasBoxGameState);
}

fn astrolabe(_: &mut i32, _: &mut ActionQueue, state: &mut GameStateManager) {
    state.push_state(ChooseTransformMasterGameState {
        num_cards_remaining: 3,
        upgrade: true,
    });
}

fn bottled_flame(_: &mut i32, _: &mut ActionQueue, state: &mut GameStateManager) {
    state.push_state(ChooseBottledCardGameState {
        ty: CardType::Attack,
    });
}

fn bottled_lightning(_: &mut i32, _: &mut ActionQueue, state: &mut GameStateManager) {
    state.push_state(ChooseBottledCardGameState {
        ty: CardType::Skill,
    });
}

fn bottled_tornado(_: &mut i32, _: &mut ActionQueue, state: &mut GameStateManager) {
    state.push_state(ChooseBottledCardGameState {
        ty: CardType::Power,
    });
}

#[derive(Debug)]
pub struct TinyHouseGameState;

impl GameState for TinyHouseGameState {
    fn run(&self, game: &mut Game) {
        let cards = Rewards::gen_card_reward(game, RareCardBaseChance::Normal);
        game.rewards.add_cards(cards);
        let has_golden_idol = game.has_relic(RelicClass::GoldenIdol);
        game.rewards.add_gold(50, has_golden_idol);
        let p = random_potion_weighted(&mut game.rng);
        game.rewards.add_potion(p);
        game.state.push_state(RewardsGameState);

        game.action_queue.push_bot(UpgradeRandomInMasterAction);
        game.state.push_state(RunActionsGameState);
    }
}

fn tiny_house(_: &mut i32, _: &mut ActionQueue, state: &mut GameStateManager) {
    state.push_state(TinyHouseGameState);
}

fn empty_cage(_: &mut i32, _: &mut ActionQueue, state: &mut GameStateManager) {
    state.push_state(ChooseRemoveFromMasterGameState {
        num_cards_remaining: 2,
    });
}

fn necronomicon_unequip(_: &mut i32, queue: &mut ActionQueue) {
    queue.push_bot(TryRemoveCardFromMasterDeckAction(CardClass::Necronomicurse));
}

fn necronomicon(
    v: &mut i32,
    _: &mut ActionQueue,
    card_queue: &mut Vec<PlayCardAction>,
    play: &PlayCardAction,
) {
    if *v == 1 && play.card.borrow().class.ty() == CardType::Attack && play.cost >= 2 {
        *v = 0;
        card_queue.push(PlayCardAction::duplicated(play));
    }
}

fn pen_nib(
    v: &mut i32,
    queue: &mut ActionQueue,
    _: &mut Vec<PlayCardAction>,
    play: &PlayCardAction,
) {
    if play.card.borrow().class.ty() == CardType::Attack {
        inc_wrap(v, 10);
        if *v == 9 {
            queue.push_bot(GainStatusAction {
                status: Status::PenNib,
                amount: 1,
                target: CreatureRef::player(),
            });
        }
    }
}

fn kunai(
    v: &mut i32,
    queue: &mut ActionQueue,
    _: &mut Vec<PlayCardAction>,
    play_card: &PlayCardAction,
) {
    if play_card.card.borrow().class.ty() == CardType::Attack && inc_wrap(v, 3) {
        queue.push_bot(GainStatusAction {
            status: Status::Dexterity,
            amount: 1,
            target: CreatureRef::player(),
        });
    }
}

fn shruiken(
    v: &mut i32,
    queue: &mut ActionQueue,
    _: &mut Vec<PlayCardAction>,
    play_card: &PlayCardAction,
) {
    if play_card.card.borrow().class.ty() == CardType::Attack && inc_wrap(v, 3) {
        queue.push_bot(GainStatusAction {
            status: Status::Strength,
            amount: 1,
            target: CreatureRef::player(),
        });
    }
}

fn ink_bottle(
    v: &mut i32,
    queue: &mut ActionQueue,
    _: &mut Vec<PlayCardAction>,
    _: &PlayCardAction,
) {
    if inc_wrap(v, 10) {
        queue.push_bot(DrawAction(1));
    }
}

fn mummified_hand(
    _: &mut i32,
    queue: &mut ActionQueue,
    _: &mut Vec<PlayCardAction>,
    play: &PlayCardAction,
) {
    if play.card.borrow().class.ty() == CardType::Power {
        queue.push_bot(DiscountRandomCardInHandAction());
    }
}

fn orange_pellets(
    v: &mut i32,
    queue: &mut ActionQueue,
    _: &mut Vec<PlayCardAction>,
    play: &PlayCardAction,
) {
    let mask = match play.card.borrow().class.ty() {
        CardType::Attack => 0b001,
        CardType::Skill => 0b010,
        CardType::Power => 0b100,
        _ => 0,
    };
    *v |= mask;
    if *v == 0b111 {
        *v = 0;
        queue.push_bot(RemoveAllDebuffsAction());
    }
}

fn ornamental_fan(
    v: &mut i32,
    queue: &mut ActionQueue,
    _: &mut Vec<PlayCardAction>,
    play: &PlayCardAction,
) {
    if play.card.borrow().class.ty() == CardType::Attack && inc_wrap(v, 3) {
        queue.push_bot(BlockAction::player_flat_amount(4));
    }
}

fn nunchaku(
    v: &mut i32,
    queue: &mut ActionQueue,
    _: &mut Vec<PlayCardAction>,
    play: &PlayCardAction,
) {
    if play.card.borrow().class.ty() == CardType::Attack && inc_wrap(v, 10) {
        queue.push_bot(GainEnergyAction(1));
    }
}

fn bird_faced_urn(
    _: &mut i32,
    queue: &mut ActionQueue,
    _: &mut Vec<PlayCardAction>,
    play: &PlayCardAction,
) {
    if play.card.borrow().class.ty() == CardType::Power {
        queue.push_bot(HealAction {
            target: CreatureRef::player(),
            amount: 2,
        });
    }
}

fn letter_opener(
    v: &mut i32,
    queue: &mut ActionQueue,
    _: &mut Vec<PlayCardAction>,
    play: &PlayCardAction,
) {
    if play.card.borrow().class.ty() == CardType::Skill && inc_wrap(v, 3) {
        queue.push_bot(DamageAllMonstersAction::thorns(5));
    }
}

fn meat_on_the_bone(_: &mut i32, queue: &mut ActionQueue) {
    // push_top is intentional
    queue.push_top(MeatOnTheBoneAction(12));
}

fn burning_blood(_: &mut i32, queue: &mut ActionQueue) {
    queue.push_bot(HealAction {
        target: CreatureRef::player(),
        amount: 6,
    });
}

fn black_blood(_: &mut i32, queue: &mut ActionQueue) {
    queue.push_bot(HealAction {
        target: CreatureRef::player(),
        amount: 12,
    });
}

fn face_of_cleric(_: &mut i32, queue: &mut ActionQueue) {
    queue.push_bot(IncreaseMaxHPAction(1));
}

fn neows_lament(v: &mut i32, queue: &mut ActionQueue) {
    // push_top is intentional
    if *v != 0 {
        *v -= 1;
        queue.push_top(SetHPAllMonstersAction(1));
    }
}

fn blood_vial(_: &mut i32, queue: &mut ActionQueue) {
    // push_top is intentional
    queue.push_top(HealAction {
        target: CreatureRef::player(),
        amount: 2,
    });
}

fn lantern(_: &mut i32, queue: &mut ActionQueue) {
    queue.push_bot(GainEnergyAction(1));
}

fn ancient_tea_set(v: &mut i32, queue: &mut ActionQueue) {
    if *v != 0 {
        *v = 0;
        queue.push_bot(GainEnergyAction(2));
    }
}

fn red_mask(_: &mut i32, queue: &mut ActionQueue) {
    queue.push_bot(GainStatusAllMonstersAction {
        status: Status::Weak,
        amount: 1,
    });
}

fn bag_of_marbles(_: &mut i32, queue: &mut ActionQueue) {
    queue.push_bot(GainStatusAllMonstersAction {
        status: Status::Vulnerable,
        amount: 1,
    });
}

fn akabeko(_: &mut i32, queue: &mut ActionQueue) {
    queue.push_bot(GainStatusAction {
        status: Status::Vigor,
        amount: 8,
        target: CreatureRef::player(),
    });
}

fn bronze_scales(_: &mut i32, queue: &mut ActionQueue) {
    // push_top is intentional
    queue.push_top(GainStatusAction {
        status: Status::Thorns,
        amount: 3,
        target: CreatureRef::player(),
    });
}

fn du_vu_doll(_: &mut i32, queue: &mut ActionQueue) {
    queue.push_bot(DuvuAction());
}

fn fossilized_helix(_: &mut i32, queue: &mut ActionQueue) {
    queue.push_bot(GainStatusAction {
        status: Status::Buffer,
        amount: 1,
        target: CreatureRef::player(),
    });
}

fn thread_and_needle(_: &mut i32, queue: &mut ActionQueue) {
    queue.push_bot(GainStatusAction {
        status: Status::PlatedArmor,
        amount: 4,
        target: CreatureRef::player(),
    });
}

fn red_skull(_: &mut i32, queue: &mut ActionQueue) {
    queue.push_bot(RedSkullAction());
}

fn toolbox(_: &mut i32, queue: &mut ActionQueue) {
    queue.push_bot(ChooseDiscoveryAction {
        ty: ChooseDiscoveryType::Colorless,
        amount: 1,
        is_free: false,
    });
}

fn pantograph(_: &mut i32, queue: &mut ActionQueue) {
    queue.push_bot(PantographAction());
}

fn vajra(_: &mut i32, queue: &mut ActionQueue) {
    queue.push_bot(GainStatusAction {
        status: Status::Strength,
        amount: 1,
        target: CreatureRef::player(),
    });
}

fn girya(v: &mut i32, queue: &mut ActionQueue) {
    queue.push_bot(GainStatusAction {
        status: Status::Strength,
        amount: *v,
        target: CreatureRef::player(),
    });
}

fn oddly_smooth_stone(_: &mut i32, queue: &mut ActionQueue) {
    queue.push_bot(GainStatusAction {
        status: Status::Dexterity,
        amount: 1,
        target: CreatureRef::player(),
    });
}

fn gremlin_visage(_: &mut i32, queue: &mut ActionQueue) {
    queue.push_bot(GainStatusAction {
        status: Status::Weak,
        amount: 1,
        target: CreatureRef::player(),
    });
}

fn clockwork_souvenir(_: &mut i32, queue: &mut ActionQueue) {
    // push_top is intentional
    queue.push_top(GainStatusAction {
        status: Status::Artifact,
        amount: 1,
        target: CreatureRef::player(),
    });
}

fn mutagenic_strength(_: &mut i32, queue: &mut ActionQueue) {
    // push_top is intentional
    queue.push_top(GainStatusAction {
        status: Status::LoseStrength,
        amount: 3,
        target: CreatureRef::player(),
    });
    queue.push_top(GainStatusAction {
        status: Status::Strength,
        amount: 3,
        target: CreatureRef::player(),
    });
}

fn preserved_insect(_: &mut i32, queue: &mut ActionQueue) {
    queue.push_bot(PreservedInsectAction());
}

fn sling_of_courage(_: &mut i32, queue: &mut ActionQueue) {
    queue.push_bot(SlingOfCourageAction());
}

fn mark_of_pain(_: &mut i32, queue: &mut ActionQueue) {
    for _ in 0..2 {
        queue.push_bot(ShuffleCardIntoDrawAction {
            class: CardClass::Wound,
            is_free: false,
        });
    }
}

fn bag_of_prep(_: &mut i32, queue: &mut ActionQueue) {
    queue.push_bot(DrawAction(2));
}

fn anchor(_: &mut i32, queue: &mut ActionQueue) {
    queue.push_bot(BlockAction::player_flat_amount(10));
}

fn horn_cleat(v: &mut i32, queue: &mut ActionQueue) {
    *v += 1;
    if *v == 2 {
        queue.push_bot(BlockAction::player_flat_amount(14));
    }
}

fn captains_wheel(v: &mut i32, queue: &mut ActionQueue) {
    *v += 1;
    if *v == 3 {
        queue.push_bot(BlockAction::player_flat_amount(18));
    }
}

fn stone_calendar(v: &mut i32, queue: &mut ActionQueue) {
    *v += 1;
    if *v == 7 {
        queue.push_bot(DamageAllMonstersAction::thorns(52));
    }
}

fn orichalcum(_: &mut i32, queue: &mut ActionQueue) {
    // push_top is intentional
    queue.push_top(OrichalcumAction(6));
}

fn nilrys_codex(_: &mut i32, queue: &mut ActionQueue) {
    queue.push_bot(ChooseCardToShuffleIntoDrawAction());
}

fn happy_flower(v: &mut i32, queue: &mut ActionQueue) {
    if inc_wrap(v, 3) {
        queue.push_bot(GainEnergyAction(1));
    }
}

fn incense_burner(v: &mut i32, queue: &mut ActionQueue) {
    if inc_wrap(v, 6) {
        queue.push_bot(GainStatusAction {
            status: Status::Intangible,
            amount: 1,
            target: CreatureRef::player(),
        });
    }
}

fn mercury_hourglass(_: &mut i32, queue: &mut ActionQueue) {
    queue.push_bot(DamageAllMonstersAction::thorns(3));
}

fn brimstone(_: &mut i32, queue: &mut ActionQueue) {
    // intentional push_top
    queue.push_top(GainStatusAction {
        status: Status::Strength,
        amount: 2,
        target: CreatureRef::player(),
    });
    queue.push_top(GainStatusAllMonstersAction {
        status: Status::Strength,
        amount: 1,
    });
}

fn art_of_war_card_played(
    v: &mut i32,
    _: &mut ActionQueue,
    _: &mut Vec<PlayCardAction>,
    play: &PlayCardAction,
) {
    if play.card.borrow().class.ty() == CardType::Attack {
        *v = 0;
    }
}

fn art_of_war(v: &mut i32, queue: &mut ActionQueue) {
    if *v == 1 {
        queue.push_bot(GainEnergyAction(1));
    }
    *v = 1;
}

fn blue_candle(
    _: &mut i32,
    queue: &mut ActionQueue,
    _: &mut Vec<PlayCardAction>,
    play: &PlayCardAction,
) {
    if play.card.borrow().class.ty() == CardType::Curse {
        queue.push_bot(DamageAction::lose_hp(1, CreatureRef::player()));
    }
}

fn abacus(_: &mut i32, queue: &mut ActionQueue) {
    // intentionally push_top, see ShuffleDiscardOnTopOfDrawAction
    queue.push_top(BlockAction::player_flat_amount(6));
}

fn sundial(v: &mut i32, queue: &mut ActionQueue) {
    if inc_wrap(v, 3) {
        // intentionally push_top, see ShuffleDiscardOnTopOfDrawAction
        queue.push_top(GainEnergyAction(2));
    }
}

fn potion_belt(_: &mut i32, queue: &mut ActionQueue, _: &mut GameStateManager) {
    queue.push_bot(IncreasePotionSlotsAction(2));
}

fn old_coin(_: &mut i32, queue: &mut ActionQueue, _: &mut GameStateManager) {
    queue.push_bot(GainGoldAction(300));
}

fn lees_waffle(_: &mut i32, queue: &mut ActionQueue, _: &mut GameStateManager) {
    queue.push_bot(IncreaseMaxHPAction(7));
    queue.push_bot(HealAction {
        target: CreatureRef::player(),
        amount: 9999,
    });
}

fn mango(_: &mut i32, queue: &mut ActionQueue, _: &mut GameStateManager) {
    queue.push_bot(IncreaseMaxHPAction(14));
}

fn pear(_: &mut i32, queue: &mut ActionQueue, _: &mut GameStateManager) {
    queue.push_bot(IncreaseMaxHPAction(10));
}

fn strawberry(_: &mut i32, queue: &mut ActionQueue, _: &mut GameStateManager) {
    queue.push_bot(IncreaseMaxHPAction(7));
}

fn enchiridion(_: &mut i32, queue: &mut ActionQueue) {
    queue.push_bot(EnchiridionAction());
}

fn snecko_eye_confused(_: &mut i32, queue: &mut ActionQueue) {
    queue.push_bot(GainStatusAction {
        status: Status::Confusion,
        amount: 1,
        target: CreatureRef::player(),
    });
}

fn snecko_eye_equip(_: &mut i32, queue: &mut ActionQueue, _: &mut GameStateManager) {
    queue.push_bot(IncreaseDrawPerTurnAction(2));
}

fn snecko_eye_unequip(_: &mut i32, queue: &mut ActionQueue) {
    queue.push_bot(IncreaseDrawPerTurnAction(-2));
}

fn war_paint(_: &mut i32, queue: &mut ActionQueue, _: &mut GameStateManager) {
    queue.push_bot(UpgradeTwoRandomInMasterAction(Some(CardType::Skill)));
}

fn whetstone(_: &mut i32, queue: &mut ActionQueue, _: &mut GameStateManager) {
    queue.push_bot(UpgradeTwoRandomInMasterAction(Some(CardType::Attack)));
}

#[derive(Clone)]
pub struct Relic {
    class: RelicClass,
    value: i32,
}

impl Relic {
    pub fn get_class(&self) -> RelicClass {
        self.class
    }
    pub fn get_value(&self) -> i32 {
        self.value
    }
    pub fn set_value(&mut self, v: i32) {
        self.value = v;
    }
}

macro_rules! trigger {
    ($name:ident) => {
        pub fn $name(&mut self, queue: &mut ActionQueue) {
            if let Some(f) = self.class.$name() {
                f(&mut self.value, queue)
            }
        }
    };
}
macro_rules! trigger_equip {
    ($name:ident) => {
        pub fn $name(&mut self, queue: &mut ActionQueue, state: &mut GameStateManager) {
            if let Some(f) = self.class.$name() {
                f(&mut self.value, queue, state)
            }
        }
    };
}
macro_rules! trigger_card {
    ($name:ident) => {
        pub fn $name(
            &mut self,
            queue: &mut ActionQueue,
            card_queue: &mut Vec<PlayCardAction>,
            play: &PlayCardAction,
        ) {
            if let Some(f) = self.class.$name() {
                f(&mut self.value, queue, card_queue, play)
            }
        }
    };
}

impl Relic {
    trigger_equip!(on_equip);
    trigger!(on_unequip);
    trigger!(on_shuffle);
    trigger!(at_pre_combat);
    trigger!(at_combat_begin_pre_draw);
    trigger!(at_combat_begin_post_draw);
    trigger!(at_turn_begin_pre_draw);
    trigger!(at_turn_begin_post_draw);
    trigger!(at_turn_end);
    trigger!(on_lose_hp);
    trigger!(at_combat_finish);
    trigger_card!(on_card_played);
}

pub fn new_relic(class: RelicClass) -> Relic {
    Relic { class, value: 0 }
}

lazy_static! {
    static ref ALL_COMMON: Vec<RelicClass> = RelicClass::all()
        .into_iter()
        .filter(|r| r.rarity() == RelicRarity::Common)
        .collect();
    static ref ALL_UNCOMMON: Vec<RelicClass> = RelicClass::all()
        .into_iter()
        .filter(|r| r.rarity() == RelicRarity::Uncommon)
        .collect();
    static ref ALL_RARE: Vec<RelicClass> = RelicClass::all()
        .into_iter()
        .filter(|r| r.rarity() == RelicRarity::Rare)
        .collect();
    static ref ALL_SHOP: Vec<RelicClass> = RelicClass::all()
        .into_iter()
        .filter(|r| r.rarity() == RelicRarity::Shop)
        .collect();
    static ref ALL_BOSS: Vec<RelicClass> = RelicClass::all()
        .into_iter()
        .filter(|r| r.rarity() == RelicRarity::Boss)
        .collect();
}

pub fn all_common_relics() -> Vec<RelicClass> {
    ALL_COMMON.clone()
}

pub fn all_uncommon_relics() -> Vec<RelicClass> {
    ALL_UNCOMMON.clone()
}

pub fn all_rare_relics() -> Vec<RelicClass> {
    ALL_RARE.clone()
}

pub fn all_shop_relics() -> Vec<RelicClass> {
    ALL_SHOP.clone()
}

pub fn all_boss_relics() -> Vec<RelicClass> {
    ALL_SHOP.clone()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        actions::{
            add_card_class_to_master_deck::AddCardClassToMasterDeckAction,
            add_card_to_master_deck::AddCardToMasterDeckAction,
            block::BlockAction,
            choose_card_to_shuffle_into_draw::ChooseCardToShuffleIntoDrawStep,
            choose_gamble::{GambleEndStep, GambleStep},
            gain_relic::GainRelicAction,
        },
        assert_matches, assert_not_matches,
        campfire::CampfireRestStep,
        cards::{CardClass, CardColor, CardRarity},
        combat::{EndTurnStep, PlayCardStep},
        game::{AscendStep, GameBuilder, GameStatus},
        map::{Map, RoomType},
        master_deck::{
            ChooseBottledCardStep, ChooseRemoveFromMasterStep, ChooseTransformMasterStep,
            DuplicateCardInMasterStep,
        },
        monsters::test::{ApplyStatusMonster, AttackMonster, NoopMonster},
        potion::Potion,
        rewards::RewardExitStep,
        state::ContinueStep,
        status::Status,
        step::Step,
    };

    #[test]
    fn test_burning_blood() {
        let mut g = GameBuilder::default()
            .add_card(CardClass::DebugKill)
            .add_relic(RelicClass::BurningBlood)
            .build_combat();
        let hp = g.player.cur_hp;
        g.step_test(PlayCardStep {
            hand_index: 0,
            target: Some(0),
        });
        assert_eq!(g.player.cur_hp, hp + 6);
    }

    #[test]
    fn test_black_blood() {
        let mut g = GameBuilder::default()
            .add_card(CardClass::DebugKill)
            .add_relic(RelicClass::BlackBlood)
            .build_combat();
        g.player.cur_hp = 10;
        g.step_test(PlayCardStep {
            hand_index: 0,
            target: Some(0),
        });
        assert_eq!(g.player.cur_hp, 10 + 12);
    }

    #[test]
    fn test_face_of_cleric() {
        let mut g = GameBuilder::default()
            .add_relic(RelicClass::FaceOfCleric)
            .build_combat();
        let hp = g.player.max_hp;
        g.play_card(CardClass::DebugKillAll, None);
        assert_eq!(g.player.max_hp, hp + 1);
    }

    #[test]
    fn test_blood_vial() {
        let g = GameBuilder::default()
            .add_relic(RelicClass::BloodVial)
            .set_player_hp(50)
            .build_combat();
        assert_eq!(g.player.cur_hp, 52);
    }

    #[test]
    fn test_bag_of_prep() {
        let g = GameBuilder::default()
            .ironclad_starting_deck()
            .add_relic(RelicClass::BagOfPrep)
            .build_combat();
        assert_eq!(g.hand.len(), 7);
    }

    #[test]
    fn test_medical_kit() {
        let mut g = GameBuilder::default()
            .add_card(CardClass::Wound)
            .add_card(CardClass::Dazed)
            .add_relic(RelicClass::MedicalKit)
            .set_player_hp(50)
            .build_combat();
        g.step_test(PlayCardStep {
            hand_index: 0,
            target: None,
        });
        g.step_test(PlayCardStep {
            hand_index: 0,
            target: None,
        });
        assert_eq!(g.energy, 3);
        assert_eq!(g.player.cur_hp, 50);
        assert_eq!(g.exhaust_pile.len(), 2);
        assert_eq!(g.discard_pile.len(), 0);
    }

    #[test]
    fn test_blue_candle() {
        let mut g = GameBuilder::default()
            .add_card(CardClass::AscendersBane)
            .add_card(CardClass::Injury)
            .add_relic(RelicClass::BlueCandle)
            .set_player_hp(50)
            .build_combat();
        g.run_action(BlockAction::player_flat_amount(5));
        g.step_test(PlayCardStep {
            hand_index: 0,
            target: None,
        });
        g.step_test(PlayCardStep {
            hand_index: 0,
            target: None,
        });
        assert_eq!(g.energy, 3);
        assert_eq!(g.player.cur_hp, 48);
        assert_eq!(g.exhaust_pile.len(), 2);
        assert_eq!(g.discard_pile.len(), 0);
    }

    #[test]
    fn test_anchor() {
        let mut g = GameBuilder::default()
            .add_relic(RelicClass::Anchor)
            .build_combat();
        assert_eq!(g.player.block, 10);
        g.step_test(EndTurnStep);
        assert_eq!(g.player.block, 0);
    }

    #[test]
    fn test_anchor_dexterity() {
        let mut g = GameBuilder::default()
            .add_relic(RelicClass::Anchor)
            .add_player_status(Status::Dexterity, 55)
            .build_combat();
        assert_eq!(g.player.block, 10);
        g.step_test(EndTurnStep);
        assert_eq!(g.player.block, 0);
    }

    #[test]
    fn test_horn_cleat() {
        let mut g = GameBuilder::default()
            .add_relic(RelicClass::HornCleat)
            .build_combat();
        assert_eq!(g.player.block, 0);
        g.step_test(EndTurnStep);
        assert_eq!(g.player.block, 14);
        g.step_test(EndTurnStep);
        assert_eq!(g.player.block, 0);
    }

    #[test]
    fn test_captains_wheel() {
        let mut g = GameBuilder::default()
            .add_relic(RelicClass::CaptainsWheel)
            .build_combat();
        assert_eq!(g.player.block, 0);
        g.step_test(EndTurnStep);
        assert_eq!(g.player.block, 0);
        g.step_test(EndTurnStep);
        assert_eq!(g.player.block, 18);
        g.step_test(EndTurnStep);
        assert_eq!(g.player.block, 0);
    }

    #[test]
    fn test_kunai() {
        let mut g = GameBuilder::default()
            .add_relic(RelicClass::Kunai)
            .build_combat();
        g.play_card(CardClass::Anger, Some(CreatureRef::monster(0)));
        g.play_card(CardClass::Anger, Some(CreatureRef::monster(0)));
        assert_eq!(g.player.get_status(Status::Dexterity), None);
        g.play_card(CardClass::Anger, Some(CreatureRef::monster(0)));
        assert_eq!(g.player.get_status(Status::Dexterity), Some(1));
        g.play_card(CardClass::Anger, Some(CreatureRef::monster(0)));
        g.step_test(EndTurnStep);
        assert_eq!(g.player.get_status(Status::Dexterity), Some(1));
        g.play_card(CardClass::Anger, Some(CreatureRef::monster(0)));
        g.play_card(CardClass::Anger, Some(CreatureRef::monster(0)));
        assert_eq!(g.player.get_status(Status::Dexterity), Some(1));
        g.play_card(CardClass::Anger, Some(CreatureRef::monster(0)));
        assert_eq!(g.player.get_status(Status::Dexterity), Some(2));
    }

    #[test]
    fn test_shruiken() {
        let mut g = GameBuilder::default()
            .add_relic(RelicClass::Shruiken)
            .build_combat();
        g.play_card(CardClass::Anger, Some(CreatureRef::monster(0)));
        g.play_card(CardClass::Anger, Some(CreatureRef::monster(0)));
        g.play_card(CardClass::GoodInstincts, Some(CreatureRef::monster(0)));
        assert_eq!(g.player.get_status(Status::Strength), None);
        g.play_card(CardClass::Anger, Some(CreatureRef::monster(0)));
        assert_eq!(g.player.get_status(Status::Strength), Some(1));
        g.play_card(CardClass::Anger, Some(CreatureRef::monster(0)));
        g.play_card(CardClass::GoodInstincts, Some(CreatureRef::monster(0)));
        g.play_card(CardClass::GoodInstincts, Some(CreatureRef::monster(0)));
        g.play_card(CardClass::GoodInstincts, Some(CreatureRef::monster(0)));
        g.step_test(EndTurnStep);
        assert_eq!(g.player.get_status(Status::Strength), Some(1));
        g.play_card(CardClass::Anger, Some(CreatureRef::monster(0)));
        g.play_card(CardClass::Berserk, Some(CreatureRef::monster(0)));
        g.play_card(CardClass::Anger, Some(CreatureRef::monster(0)));
        assert_eq!(g.player.get_status(Status::Strength), Some(1));
        g.play_card(CardClass::Anger, Some(CreatureRef::monster(0)));
        assert_eq!(g.player.get_status(Status::Strength), Some(2));
    }

    #[test]
    fn test_ink_bottle() {
        let mut g = GameBuilder::default()
            .add_relic(RelicClass::InkBottle)
            .build_combat();
        g.add_card_to_draw_pile(CardClass::Strike);
        for i in 0..9 {
            g.play_card(CardClass::Bloodletting, None);
            assert_eq!(g.get_relic_value(RelicClass::InkBottle), Some(i + 1));
            assert_eq!(g.hand.len(), 0);
        }
        g.play_card(CardClass::Bloodletting, None);
        assert_eq!(g.get_relic_value(RelicClass::InkBottle), Some(0));
        assert_eq!(g.hand.len(), 1);
        g.play_card(CardClass::Bloodletting, None);
        assert_eq!(g.get_relic_value(RelicClass::InkBottle), Some(1));
        g.step_test(EndTurnStep);
        assert_eq!(g.get_relic_value(RelicClass::InkBottle), Some(1));
    }

    #[test]
    fn test_strike_dummy() {
        let mut g = GameBuilder::default()
            .add_relic(RelicClass::StrikeDummy)
            .build_combat();
        g.energy = 99;
        let hp = g.monsters[0].creature.cur_hp;
        g.play_card(CardClass::Strike, Some(CreatureRef::monster(0)));
        assert_eq!(g.monsters[0].creature.cur_hp, hp - 9);
        g.play_card(CardClass::TwinStrike, Some(CreatureRef::monster(0)));
        assert_eq!(g.monsters[0].creature.cur_hp, hp - (6 + 3) - (5 + 3) * 2);
        g.play_card(CardClass::Bash, Some(CreatureRef::monster(0)));
        assert_eq!(
            g.monsters[0].creature.cur_hp,
            hp - (6 + 3) - (5 + 3) * 2 - 8
        );
    }

    #[test]
    fn test_letter_opener() {
        let mut g = GameBuilder::default()
            .add_relic(RelicClass::LetterOpener)
            .build_combat_with_monsters(NoopMonster::with_hp(50), NoopMonster::with_hp(50));
        g.energy = 99;
        g.play_card(CardClass::Strike, Some(CreatureRef::monster(0)));
        g.play_card(CardClass::Defend, None);
        g.play_card(CardClass::Defend, None);
        assert_eq!(g.get_relic_value(RelicClass::LetterOpener), Some(2));
        assert_eq!(g.monsters[0].creature.cur_hp, 50 - 6);
        assert_eq!(g.monsters[1].creature.cur_hp, 50);
        g.play_card(CardClass::Defend, None);
        assert_eq!(g.get_relic_value(RelicClass::LetterOpener), Some(0));
        assert_eq!(g.monsters[0].creature.cur_hp, 50 - 6 - 5);
        assert_eq!(g.monsters[1].creature.cur_hp, 50 - 5);
        g.play_card(CardClass::Defend, None);
        assert_eq!(g.get_relic_value(RelicClass::LetterOpener), Some(1));
        g.step_test(EndTurnStep);
        assert_eq!(g.get_relic_value(RelicClass::LetterOpener), Some(0));
    }

    #[test]
    fn test_ornamental_fan() {
        let mut g = GameBuilder::default()
            .add_relic(RelicClass::OrnamentalFan)
            .build_combat();
        assert_eq!(g.get_relic_value(RelicClass::OrnamentalFan), Some(0));
        g.play_card(CardClass::Intimidate, None);
        g.play_card(CardClass::Whirlwind, None);
        assert_eq!(g.get_relic_value(RelicClass::OrnamentalFan), Some(1));
        g.play_card(CardClass::Whirlwind, None);
        assert_eq!(g.get_relic_value(RelicClass::OrnamentalFan), Some(2));
        assert_eq!(g.player.block, 0);
        g.play_card(CardClass::Whirlwind, None);
        assert_eq!(g.get_relic_value(RelicClass::OrnamentalFan), Some(0));
        assert_eq!(g.player.block, 4);
        g.play_card(CardClass::Intimidate, None);
        assert_eq!(g.get_relic_value(RelicClass::OrnamentalFan), Some(0));
        g.play_card(CardClass::Whirlwind, None);
        g.step_test(EndTurnStep);
        assert_eq!(g.get_relic_value(RelicClass::OrnamentalFan), Some(0));
    }

    #[test]
    fn test_abacus() {
        let mut g = GameBuilder::default()
            .add_relic(RelicClass::TheAbacus)
            .build_combat();
        assert_eq!(g.player.block, 0);
        g.play_card(CardClass::Thunderclap, Some(CreatureRef::monster(0)));
        assert_eq!(g.player.block, 0);
        g.play_card(CardClass::PommelStrike, Some(CreatureRef::monster(0)));
        assert_eq!(g.player.block, 6);
        g.play_card(CardClass::PommelStrike, Some(CreatureRef::monster(0)));
        assert_eq!(g.player.block, 12);
        g.play_card(CardClass::MasterOfStrategy, Some(CreatureRef::monster(0)));
        assert_eq!(g.player.block, 24);
        g.play_card(CardClass::MasterOfStrategy, Some(CreatureRef::monster(0)));
        assert_eq!(g.player.block, 24);
    }

    #[test]
    fn test_sundial() {
        let mut g = GameBuilder::default()
            .add_relic(RelicClass::Sundial)
            .build_combat();

        g.add_cards_to_discard_pile(CardClass::Strike, 10);
        assert_eq!(g.get_relic_value(RelicClass::Sundial), Some(0));
        g.play_card(CardClass::FlashOfSteel, Some(CreatureRef::monster(0)));
        assert_eq!(g.get_relic_value(RelicClass::Sundial), Some(1));
        assert_eq!(g.energy, 3);

        g.clear_all_piles();
        g.add_cards_to_discard_pile(CardClass::Strike, 10);
        g.play_card(CardClass::FlashOfSteel, Some(CreatureRef::monster(0)));
        assert_eq!(g.get_relic_value(RelicClass::Sundial), Some(2));
        assert_eq!(g.energy, 3);

        g.clear_all_piles();
        g.add_cards_to_discard_pile(CardClass::Strike, 10);
        g.play_card(CardClass::FlashOfSteel, Some(CreatureRef::monster(0)));
        assert_eq!(g.get_relic_value(RelicClass::Sundial), Some(0));
        assert_eq!(g.energy, 5);

        g.clear_all_piles();
        g.add_cards_to_discard_pile(CardClass::Strike, 10);
        g.play_card(CardClass::FlashOfSteel, Some(CreatureRef::monster(0)));
        assert_eq!(g.get_relic_value(RelicClass::Sundial), Some(1));
        assert_eq!(g.energy, 5);

        g.step_test(EndTurnStep);
        assert_eq!(g.get_relic_value(RelicClass::Sundial), Some(1));
    }

    #[test]
    fn test_sundial_infinite() {
        {
            let mut g = GameBuilder::default()
                .add_relic(RelicClass::Sundial)
                .add_cards_upgraded(CardClass::PommelStrike, 2)
                .build_combat_with_monster(NoopMonster::with_hp(10000));
            for _ in 0..100 {
                g.step_test(PlayCardStep {
                    hand_index: 0,
                    target: Some(0),
                });
            }
            assert!(g.energy > 10);
        }
        {
            let mut g = GameBuilder::default()
                .add_relic(RelicClass::Sundial)
                .add_card(CardClass::PommelStrike)
                .add_card_upgraded(CardClass::PommelStrike)
                .build_combat_with_monster(NoopMonster::with_hp(10000));
            for _ in 0..100 {
                g.step_test(PlayCardStep {
                    hand_index: 0,
                    target: Some(0),
                });
            }
            assert!(g.energy < 10);
        }
    }

    #[test]
    fn test_combat_begin() {
        {
            let g = GameBuilder::default()
                .add_relic(RelicClass::MutagenicStrength)
                .add_relic(RelicClass::ClockworkSouvenir)
                .add_relic(RelicClass::GremlinVisage)
                .build_combat();
            assert_eq!(g.player.get_status(Status::Weak), Some(1));
            assert_eq!(g.player.get_status(Status::Artifact), None);
            assert_eq!(g.player.get_status(Status::Strength), Some(3));
            assert_eq!(g.player.get_status(Status::LoseStrength), None);
        }
        {
            let g = GameBuilder::default()
                .add_relic(RelicClass::ClockworkSouvenir)
                .add_relic(RelicClass::MutagenicStrength)
                .add_relic(RelicClass::GremlinVisage)
                .build_combat();
            assert_eq!(g.player.get_status(Status::Weak), None);
            assert_eq!(g.player.get_status(Status::Artifact), None);
            assert_eq!(g.player.get_status(Status::Strength), Some(3));
            assert_eq!(g.player.get_status(Status::LoseStrength), Some(3));
        }
        {
            let g = GameBuilder::default()
                .add_relic(RelicClass::GremlinVisage)
                .add_relic(RelicClass::ClockworkSouvenir)
                .add_relic(RelicClass::MutagenicStrength)
                .build_combat();
            assert_eq!(g.player.get_status(Status::Weak), None);
            assert_eq!(g.player.get_status(Status::Artifact), None);
            assert_eq!(g.player.get_status(Status::Strength), Some(3));
            assert_eq!(g.player.get_status(Status::LoseStrength), Some(3));
        }
    }

    #[test]
    fn test_snecko_eye() {
        let mut g = GameBuilder::default()
            .add_relic(RelicClass::ClockworkSouvenir)
            .add_relic(RelicClass::SneckoEye)
            .add_cards(CardClass::Strike, 10)
            .build_combat();
        assert_eq!(g.player.get_status(Status::Confusion), Some(1));
        assert_eq!(g.hand.len(), 7);
        g.remove_relic(RelicClass::SneckoEye);
        g.step_test(EndTurnStep);
        assert_eq!(g.hand.len(), 5);
    }

    #[test]
    fn test_lantern() {
        let mut g = GameBuilder::default()
            .add_relic(RelicClass::Lantern)
            .build_combat();
        assert_eq!(g.energy, 4);
        g.step_test(EndTurnStep);
        assert_eq!(g.energy, 3);
    }

    #[test]
    fn test_ancient_tea_set() {
        let mut g = GameBuilder::default()
            .add_relic(RelicClass::AncientTeaSet)
            .build_with_rooms(&[
                RoomType::Monster,
                RoomType::Campfire,
                RoomType::Monster,
                RoomType::Monster,
            ]);
        g.roll_noop_monsters = true;
        g.step_test(AscendStep::new(0, 0));
        assert_eq!(g.energy, 3);
        g.step_test(EndTurnStep);
        assert_eq!(g.energy, 3);
        g.play_card(CardClass::DebugKillAll, None);
        g.step_test(RewardExitStep);
        g.step_test(AscendStep::new(0, 1));
        g.step_test(CampfireRestStep);
        g.step_test(AscendStep::new(0, 2));
        assert_eq!(g.energy, 5);
        g.step_test(EndTurnStep);
        assert_eq!(g.energy, 3);
        g.play_card(CardClass::DebugKillAll, None);
        g.step_test(RewardExitStep);
        g.step_test(AscendStep::new(0, 3));
        assert_eq!(g.energy, 3);
    }

    #[test]
    fn test_happy_flower() {
        let mut g = GameBuilder::default()
            .add_relic(RelicClass::HappyFlower)
            .build_combat();
        assert_eq!(g.get_relic_value(RelicClass::HappyFlower), Some(1));
        assert_eq!(g.energy, 3);
        g.step_test(EndTurnStep);
        assert_eq!(g.get_relic_value(RelicClass::HappyFlower), Some(2));
        assert_eq!(g.energy, 3);
        g.step_test(EndTurnStep);
        assert_eq!(g.get_relic_value(RelicClass::HappyFlower), Some(0));
        assert_eq!(g.energy, 4);
        g.step_test(EndTurnStep);
        assert_eq!(g.get_relic_value(RelicClass::HappyFlower), Some(1));
        assert_eq!(g.energy, 3);
    }

    #[test]
    fn test_incense_burner() {
        let mut g = GameBuilder::default()
            .add_relic(RelicClass::IncenseBurner)
            .build_with_rooms(&[RoomType::Monster, RoomType::Monster]);

        g.roll_noop_monsters = true;

        g.step_test(AscendStep::new(0, 0));

        assert_eq!(g.get_relic_value(RelicClass::IncenseBurner), Some(1));
        assert_eq!(g.player.get_status(Status::Intangible), None);
        g.step_test(EndTurnStep);
        assert_eq!(g.get_relic_value(RelicClass::IncenseBurner), Some(2));
        assert_eq!(g.player.get_status(Status::Intangible), None);
        g.step_test(EndTurnStep);
        assert_eq!(g.get_relic_value(RelicClass::IncenseBurner), Some(3));
        assert_eq!(g.player.get_status(Status::Intangible), None);
        g.step_test(EndTurnStep);
        assert_eq!(g.get_relic_value(RelicClass::IncenseBurner), Some(4));
        assert_eq!(g.player.get_status(Status::Intangible), None);
        g.step_test(EndTurnStep);
        assert_eq!(g.get_relic_value(RelicClass::IncenseBurner), Some(5));
        assert_eq!(g.player.get_status(Status::Intangible), None);
        g.step_test(EndTurnStep);
        assert_eq!(g.get_relic_value(RelicClass::IncenseBurner), Some(0));
        assert_eq!(g.player.get_status(Status::Intangible), Some(1));
        g.step_test(EndTurnStep);
        assert_eq!(g.get_relic_value(RelicClass::IncenseBurner), Some(1));
        assert_eq!(g.player.get_status(Status::Intangible), None);
        g.play_card(CardClass::DebugKillAll, None);
        g.step_test(RewardExitStep);
        g.step_test(AscendStep::new(0, 1));
        assert_eq!(g.get_relic_value(RelicClass::IncenseBurner), Some(2));
    }

    #[test]
    fn test_ectoplasm() {
        let mut g = GameBuilder::default()
            .add_relic(RelicClass::Ectoplasm)
            .build_combat_with_monster(NoopMonster::with_hp(1));
        assert_eq!(g.energy, 4);
        g.play_card(CardClass::HandOfGreed, Some(CreatureRef::monster(0)));
        assert_eq!(g.gold, 0);
    }

    #[test]
    fn test_food() {
        let mut g = GameBuilder::default().build_combat();
        let max_hp = g.player.max_hp;
        g.add_relic(RelicClass::Mango);
        assert_eq!(g.player.max_hp, max_hp + 14);
        g.add_relic(RelicClass::Pear);
        assert_eq!(g.player.max_hp, max_hp + 14 + 10);
        g.add_relic(RelicClass::Strawberry);
        assert_eq!(g.player.max_hp, max_hp + 14 + 10 + 7);
        assert_ne!(g.player.max_hp, g.player.cur_hp);
        g.add_relic(RelicClass::LeesWaffle);
        assert_eq!(g.player.max_hp, max_hp + 14 + 10 + 7 + 7);
        assert_eq!(g.player.max_hp, g.player.cur_hp);
    }

    #[test]
    fn test_potion_belt() {
        let mut g = GameBuilder::default()
            .add_relic(RelicClass::PotionBelt)
            .build_combat();
        assert_eq!(g.potions, vec![None; 4]);
        g.remove_relic(RelicClass::PotionBelt);
        assert_eq!(g.potions, vec![None; 4]);
        for _ in 0..4 {
            g.add_potion(Potion::Ancient);
        }
    }

    #[test]
    fn test_war_paint() {
        {
            let mut g = GameBuilder::default().build_combat();
            g.add_relic(RelicClass::WarPaint);
        }
        {
            let mut g = GameBuilder::default()
                .add_card(CardClass::Strike)
                .add_card(CardClass::Defend)
                .add_card(CardClass::ShrugItOff)
                .add_card_upgraded(CardClass::BandageUp)
                .build_combat();
            g.add_relic(RelicClass::WarPaint);
            assert_eq!(
                g.master_deck
                    .iter()
                    .map(|c| c.borrow().upgrade_count)
                    .sum::<i32>(),
                3
            );
            assert!(g.master_deck.iter().any(
                |c| c.borrow().class == CardClass::Strike && c.borrow().upgrade_count == 0
            ));
        }
        {
            let mut found_unupgraded_defend = false;
            let mut found_unupgraded_shrug = false;
            let mut found_unupgraded_bandage = false;
            for _ in 0..100 {
                let mut g = GameBuilder::default()
                    .add_card(CardClass::Strike)
                    .add_card(CardClass::Defend)
                    .add_card(CardClass::ShrugItOff)
                    .add_card(CardClass::BandageUp)
                    .build_combat();
                g.add_relic(RelicClass::WarPaint);
                assert_eq!(
                    g.master_deck
                        .iter()
                        .map(|c| c.borrow().upgrade_count)
                        .sum::<i32>(),
                    2
                );
                assert!(g.master_deck.iter().any(
                    |c| c.borrow().class == CardClass::Strike && c.borrow().upgrade_count == 0
                ));
                for c in &g.master_deck {
                    if c.borrow().upgrade_count != 0 {
                        continue;
                    }
                    match c.borrow().class {
                        CardClass::Defend => found_unupgraded_defend = true,
                        CardClass::ShrugItOff => found_unupgraded_shrug = true,
                        CardClass::BandageUp => found_unupgraded_bandage = true,
                        _ => {}
                    }
                }
                if found_unupgraded_defend && found_unupgraded_shrug && found_unupgraded_bandage {
                    break;
                }
            }
            assert!(found_unupgraded_defend && found_unupgraded_shrug && found_unupgraded_bandage);
        }
    }

    #[test]
    fn test_whetstone() {
        {
            let mut g = GameBuilder::default().build_combat();
            g.add_relic(RelicClass::Whetstone);
        }
        {
            let mut g = GameBuilder::default()
                .add_card(CardClass::Bash)
                .build_combat();
            g.add_relic(RelicClass::Whetstone);
            assert_eq!(g.master_deck[0].borrow().upgrade_count, 1);
        }
        {
            let mut g = GameBuilder::default()
                .add_card(CardClass::Defend)
                .add_card(CardClass::Strike)
                .add_card_upgraded(CardClass::SearingBlow)
                .build_combat();
            g.add_relic(RelicClass::Whetstone);
            for c in &g.master_deck {
                let c = c.borrow();
                match c.class {
                    CardClass::Defend => assert_eq!(c.upgrade_count, 0),
                    CardClass::Strike => assert_eq!(c.upgrade_count, 1),
                    CardClass::SearingBlow => assert_eq!(c.upgrade_count, 2),
                    _ => panic!(),
                }
            }
        }
    }

    #[test]
    fn test_red_mask() {
        let g = GameBuilder::default()
            .add_relic(RelicClass::RedMask)
            .build_combat();
        assert_eq!(g.monsters[0].creature.get_status(Status::Weak), Some(1));
    }

    #[test]
    fn test_bag_of_marbles() {
        let g = GameBuilder::default()
            .add_relic(RelicClass::BagOfMarbles)
            .build_combat();
        assert_eq!(
            g.monsters[0].creature.get_status(Status::Vulnerable),
            Some(1)
        );
    }

    #[test]
    fn test_duvu_doll() {
        assert_eq!(
            GameBuilder::default()
                .add_relic(RelicClass::DuVuDoll)
                .build_combat()
                .player
                .get_status(Status::Strength),
            None
        );
        assert_eq!(
            GameBuilder::default()
                .add_card(CardClass::Strike)
                .add_relic(RelicClass::DuVuDoll)
                .build_combat()
                .player
                .get_status(Status::Strength),
            None
        );
        assert_eq!(
            GameBuilder::default()
                .add_card(CardClass::AscendersBane)
                .add_relic(RelicClass::DuVuDoll)
                .build_combat()
                .player
                .get_status(Status::Strength),
            Some(1)
        );
        assert_eq!(
            GameBuilder::default()
                .add_card(CardClass::AscendersBane)
                .add_card(CardClass::CurseOfTheBell)
                .add_relic(RelicClass::DuVuDoll)
                .build_combat()
                .player
                .get_status(Status::Strength),
            Some(2)
        );
    }

    #[test]
    fn test_vajra() {
        let g = GameBuilder::default()
            .add_relic(RelicClass::Vajra)
            .build_combat();
        assert_eq!(g.player.get_status(Status::Strength), Some(1));
    }

    #[test]
    fn test_oddly_smooth_stone() {
        let g = GameBuilder::default()
            .add_relic(RelicClass::OddlySmoothStone)
            .build_combat();
        assert_eq!(g.player.get_status(Status::Dexterity), Some(1));
    }

    #[test]
    fn test_odd_mushroom() {
        let mut g = GameBuilder::default()
            .add_relic(RelicClass::OddMushroom)
            .add_player_status(Status::Vulnerable, 5)
            .build_combat_with_monster(AttackMonster::new(10));
        let hp = g.player.cur_hp;
        g.step_test(EndTurnStep);
        assert_eq!(g.player.cur_hp, hp - 12);
    }

    #[test]
    fn test_paper_phrog() {
        let mut g = GameBuilder::default()
            .add_relic(RelicClass::PaperPhrog)
            .add_monster_status(Status::Vulnerable, 5)
            .build_combat();
        let hp = g.monsters[0].creature.cur_hp;
        g.play_card_upgraded(CardClass::Bash, Some(CreatureRef::monster(0)));
        assert_eq!(g.monsters[0].creature.cur_hp, hp - 17);
    }

    #[test]
    fn test_nunchaku() {
        let mut g = GameBuilder::default()
            .add_relic(RelicClass::Nunchaku)
            .build_combat();
        for i in 1..15 {
            g.play_card(CardClass::Anger, Some(CreatureRef::monster(0)));
            assert_eq!(g.get_relic_value(RelicClass::Nunchaku), Some(i % 10));
            if i < 10 {
                assert_eq!(g.energy, 3);
            } else {
                assert_eq!(g.energy, 4);
            }
        }
        g.step_test(EndTurnStep);
        assert_eq!(g.get_relic_value(RelicClass::Nunchaku), Some(4));
    }

    #[test]
    fn test_bird_faced_urn() {
        let mut g = GameBuilder::default()
            .add_relic(RelicClass::BirdFacedUrn)
            .build_combat();
        let hp = g.player.cur_hp;
        g.play_card(CardClass::Anger, Some(CreatureRef::monster(0)));
        assert_eq!(g.player.cur_hp, hp);
        g.play_card(CardClass::Inflame, Some(CreatureRef::monster(0)));
        assert_eq!(g.player.cur_hp, hp + 2);
    }

    #[test]
    fn test_calipers() {
        let mut g = GameBuilder::default()
            .add_relic(RelicClass::Calipers)
            .build_combat();
        g.player.block = 20;
        g.step_test(EndTurnStep);
        assert_eq!(g.player.block, 5);
        g.step_test(EndTurnStep);
        assert_eq!(g.player.block, 0);
    }

    #[test]
    fn test_ice_cream() {
        let mut g = GameBuilder::default()
            .add_relic(RelicClass::IceCream)
            .build_combat();
        assert_eq!(g.energy, 3);
        g.play_card(CardClass::Defend, None);
        assert_eq!(g.energy, 2);
        g.step_test(EndTurnStep);
        assert_eq!(g.energy, 5);
        g.add_relic(RelicClass::Ectoplasm);
        g.step_test(EndTurnStep);
        assert_eq!(g.energy, 9);
    }

    #[test]
    fn test_akabeko() {
        let mut g = GameBuilder::default()
            .add_relic(RelicClass::Akabeko)
            .build_combat();
        assert_eq!(g.player.get_status(Status::Vigor), Some(8));
        g.step_test(EndTurnStep);
        assert_eq!(g.player.get_status(Status::Vigor), Some(8));
    }

    #[test]
    fn test_art_of_war() {
        let mut g = GameBuilder::default()
            .add_relic(RelicClass::ArtOfWar)
            .build_combat();
        assert_eq!(g.energy, 3);
        assert_eq!(g.get_relic_value(RelicClass::ArtOfWar), Some(1));
        g.step_test(EndTurnStep);
        assert_eq!(g.energy, 4);
        assert_eq!(g.get_relic_value(RelicClass::ArtOfWar), Some(1));
        g.play_card(CardClass::Inflame, None);
        g.play_card(CardClass::Defend, None);
        assert_eq!(g.get_relic_value(RelicClass::ArtOfWar), Some(1));
        g.step_test(EndTurnStep);
        assert_eq!(g.energy, 4);
        assert_eq!(g.get_relic_value(RelicClass::ArtOfWar), Some(1));
        g.play_card(CardClass::Thunderclap, None);
        assert_eq!(g.get_relic_value(RelicClass::ArtOfWar), Some(0));
        g.step_test(EndTurnStep);
        assert_eq!(g.energy, 3);
    }

    #[test]
    fn test_pocketwatch() {
        let mut g = GameBuilder::default()
            .add_cards(CardClass::Strike, 10)
            .add_relic(RelicClass::Pocketwatch)
            .build_combat();

        assert_eq!(g.hand.len(), 5);
        assert_eq!(g.get_relic_value(RelicClass::Pocketwatch), Some(0));

        g.step_test(EndTurnStep);
        assert_eq!(g.hand.len(), 8);

        g.play_card(CardClass::Anger, Some(CreatureRef::monster(0)));
        g.play_card(CardClass::Anger, Some(CreatureRef::monster(0)));
        g.play_card(CardClass::Anger, Some(CreatureRef::monster(0)));
        assert_eq!(g.get_relic_value(RelicClass::Pocketwatch), Some(3));
        g.step_test(EndTurnStep);
        assert_eq!(g.hand.len(), 8);

        g.play_card(CardClass::Anger, Some(CreatureRef::monster(0)));
        g.play_card(CardClass::Anger, Some(CreatureRef::monster(0)));
        g.play_card(CardClass::Anger, Some(CreatureRef::monster(0)));
        g.play_card(CardClass::Anger, Some(CreatureRef::monster(0)));
        assert_eq!(g.get_relic_value(RelicClass::Pocketwatch), Some(4));
        g.step_test(EndTurnStep);
        assert_eq!(g.hand.len(), 5);
        assert_eq!(g.get_relic_value(RelicClass::Pocketwatch), Some(0));
    }

    #[test]
    fn test_bronze_scales() {
        let mut g = GameBuilder::default()
            .add_relic(RelicClass::BronzeScales)
            .build_combat();
        assert_eq!(g.player.get_status(Status::Thorns), Some(3));
        g.step_test(EndTurnStep);
        assert_eq!(g.player.get_status(Status::Thorns), Some(3));
    }

    #[test]
    fn test_omamori() {
        let mut g = GameBuilder::default()
            .add_relic(RelicClass::Omamori)
            .build_combat();
        assert_eq!(g.get_relic_value(RelicClass::Omamori), Some(2));

        g.add_card_to_master_deck(CardClass::Anger);
        assert_eq!(g.master_deck.len(), 1);
        assert_eq!(g.get_relic_value(RelicClass::Omamori), Some(2));

        g.add_card_to_master_deck(CardClass::CurseOfTheBell);
        assert_eq!(g.master_deck.len(), 1);
        assert_eq!(g.get_relic_value(RelicClass::Omamori), Some(1));

        g.add_card_to_master_deck(CardClass::Parasite);
        assert_eq!(g.master_deck.len(), 1);
        assert_eq!(g.get_relic_value(RelicClass::Omamori), Some(0));

        g.add_card_to_master_deck(CardClass::Parasite);
        assert_eq!(g.master_deck.len(), 2);
        assert_eq!(g.get_relic_value(RelicClass::Omamori), Some(0));
    }

    #[test]
    fn test_darkstone_periapt() {
        let mut g = GameBuilder::default()
            .add_relic(RelicClass::DarkstonePeriapt)
            .build_combat();

        let max_hp = g.player.max_hp;

        g.add_card_to_master_deck(CardClass::Anger);
        assert_eq!(g.player.max_hp, max_hp);

        g.add_card_to_master_deck(CardClass::CurseOfTheBell);
        assert_eq!(g.player.max_hp, max_hp + 6);

        g.add_card_to_master_deck(CardClass::AscendersBane);
        assert_eq!(g.player.max_hp, max_hp + 12);

        g.add_relic(RelicClass::Omamori);
        g.add_card_to_master_deck(CardClass::AscendersBane);
        assert_eq!(g.player.max_hp, max_hp + 12);
    }

    #[test]
    fn test_gremlin_horn() {
        let mut g = GameBuilder::default()
            .add_relic(RelicClass::GremlinHorn)
            .add_player_status(Status::Thorns, 999)
            .build_combat_with_monsters_3(
                NoopMonster::new(),
                AttackMonster::new(1),
                NoopMonster::new(),
            );
        g.add_cards_to_draw_pile(CardClass::Strike, 10);

        assert_eq!(g.energy, 3);
        assert_eq!(g.hand.len(), 0);

        g.play_card(CardClass::DebugKill, Some(CreatureRef::monster(0)));
        assert_eq!(g.energy, 4);
        assert_eq!(g.hand.len(), 1);

        g.step_test(EndTurnStep);
        assert_eq!(g.energy, 3);
        assert_eq!(g.hand.len(), 6);
    }

    #[test]
    fn test_pen_nib() {
        {
            let mut g = GameBuilder::default()
                .add_relic(RelicClass::PenNib)
                .build_combat();

            for i in 1..15 {
                g.play_card(CardClass::Anger, Some(CreatureRef::monster(0)));
                g.play_card(CardClass::GoodInstincts, None);
                assert_eq!(g.get_relic_value(RelicClass::PenNib), Some(i % 10));
                if i == 9 {
                    assert_eq!(g.player.get_status(Status::PenNib), Some(1));
                } else {
                    assert_eq!(g.player.get_status(Status::PenNib), None);
                }
            }
        }
        {
            let mut g = GameBuilder::default()
                .add_relic(RelicClass::PenNib)
                .build_with_rooms(&[RoomType::Monster, RoomType::Monster, RoomType::Monster]);
            g.roll_noop_monsters = true;
            g.step_test(AscendStep::new(0, 0));
            for _ in 0..8 {
                g.play_card(CardClass::Anger, Some(CreatureRef::monster(0)));
            }
            assert_eq!(g.player.get_status(Status::PenNib), None);
            g.play_card(CardClass::DebugKill, Some(CreatureRef::monster(0)));
            g.step_test(RewardExitStep);
            g.step_test(AscendStep::new(0, 1));

            assert_eq!(g.monsters[0].creature.cur_hp, g.monsters[0].creature.max_hp);
            assert_eq!(g.player.get_status(Status::PenNib), Some(1));
            assert_eq!(g.get_relic_value(RelicClass::PenNib), Some(9));
            g.play_card(CardClass::DebugKill, Some(CreatureRef::monster(0)));
            g.step_test(RewardExitStep);
            g.step_test(AscendStep::new(0, 2));

            assert_eq!(g.monsters[0].creature.cur_hp, g.monsters[0].creature.max_hp);
            assert_eq!(g.get_relic_value(RelicClass::PenNib), Some(0));
            assert_eq!(g.player.get_status(Status::PenNib), None);
        }
    }

    #[test]
    fn test_tungsten_rod() {
        let mut g = GameBuilder::default()
            .add_relic(RelicClass::TungstenRod)
            .build_combat_with_monster(AttackMonster::with_attack_count(4, 2));
        let hp = g.player.cur_hp;
        g.play_card(CardClass::Bloodletting, None);
        assert_eq!(g.player.cur_hp, hp - 2);
        g.step_test(EndTurnStep);
        assert_eq!(g.player.cur_hp, hp - 2 - 6);
        g.play_card(CardClass::Defend, None);
        g.step_test(EndTurnStep);
        assert_eq!(g.player.cur_hp, hp - 2 - 6 - 2);

        g.play_card(CardClass::Apparition, None);
        g.play_card(CardClass::Apparition, None);
        g.clear_all_piles();
        g.run_action(GainStatusAction {
            status: Status::PlatedArmor,
            amount: 1,
            target: CreatureRef::player(),
        });
        g.step_test(EndTurnStep);
        assert_eq!(g.player.cur_hp, hp - 2 - 6 - 2);
        assert_eq!(g.player.get_status(Status::PlatedArmor), Some(1));
    }

    #[test]
    fn test_blood_for_blood_tungsten_rod() {
        let mut g = GameBuilder::default()
            .add_relic(RelicClass::TungstenRod)
            .build_combat();
        g.play_card(CardClass::Apparition, None);
        g.play_card(CardClass::Bloodletting, None);
        g.add_card_to_hand(CardClass::BloodForBlood);
        assert_eq!(g.hand[0].borrow().get_base_cost(), 4);
    }

    #[test]
    fn test_tungsten_rod_buffer() {
        let mut g = GameBuilder::default()
            .add_relic(RelicClass::TungstenRod)
            .add_player_status(Status::Buffer, 1)
            .build_combat_with_monster(AttackMonster::new(1));
        let hp = g.player.cur_hp;
        g.step_test(EndTurnStep);
        assert_eq!(g.player.cur_hp, hp);
        assert_eq!(g.player.get_status(Status::Buffer), None);
    }

    #[test]
    fn test_boot() {
        let mut g = GameBuilder::default()
            .add_relic(RelicClass::Boot)
            .add_player_status(Status::Strength, -6)
            .build_combat();
        let hp = g.monsters[0].creature.cur_hp;
        g.play_card(CardClass::Strike, Some(CreatureRef::monster(0)));
        assert_eq!(g.monsters[0].creature.cur_hp, hp);
        g.play_card_upgraded(CardClass::Strike, Some(CreatureRef::monster(0)));
        assert_eq!(g.monsters[0].creature.cur_hp, hp - 5);
        g.run_action(GainStatusAction {
            status: Status::Intangible,
            amount: 4,
            target: CreatureRef::monster(0),
        });
        g.play_card_upgraded(CardClass::Strike, Some(CreatureRef::monster(0)));
        assert_eq!(g.monsters[0].creature.cur_hp, hp - 5 - 5);
    }

    #[test]
    fn test_fossilized_helix() {
        let g = GameBuilder::default()
            .add_relic(RelicClass::FossilizedHelix)
            .build_combat();
        assert_eq!(g.player.get_status(Status::Buffer), Some(1));
    }

    #[test]
    fn test_thread_and_needle() {
        let g = GameBuilder::default()
            .add_relic(RelicClass::ThreadAndNeedle)
            .build_combat();
        assert_eq!(g.player.get_status(Status::PlatedArmor), Some(4));
    }

    #[test]
    fn test_chemical_x() {
        let mut g = GameBuilder::default()
            .add_relic(RelicClass::ChemicalX)
            .build_combat();
        let hp = g.monsters[0].creature.cur_hp;
        g.play_card(CardClass::Whirlwind, None);
        assert_eq!(g.monsters[0].creature.cur_hp, hp - 5 * 5);
        g.play_card(CardClass::Transmutation, None);
        assert_eq!(g.hand.len(), 2);
    }

    #[test]
    fn test_turnip() {
        let mut g = GameBuilder::default()
            .add_relic(RelicClass::Turnip)
            .build_combat_with_monsters(
                ApplyStatusMonster {
                    status: Status::Weak,
                    amount: 2,
                },
                ApplyStatusMonster {
                    status: Status::Frail,
                    amount: 2,
                },
            );
        g.step_test(EndTurnStep);
        assert_eq!(g.player.get_status(Status::Weak), Some(2));
        assert_eq!(g.player.get_status(Status::Frail), None);
        g.player.set_status(Status::Artifact, 2);
        g.step_test(EndTurnStep);
        assert_eq!(g.player.get_status(Status::Weak), Some(1));
        assert_eq!(g.player.get_status(Status::Frail), None);
        assert_eq!(g.player.get_status(Status::Artifact), Some(1));
    }

    #[test]
    fn test_ginger() {
        let mut g = GameBuilder::default()
            .add_relic(RelicClass::Ginger)
            .build_combat_with_monsters(
                ApplyStatusMonster {
                    status: Status::Weak,
                    amount: 2,
                },
                ApplyStatusMonster {
                    status: Status::Frail,
                    amount: 2,
                },
            );
        g.step_test(EndTurnStep);
        assert_eq!(g.player.get_status(Status::Frail), Some(2));
        assert_eq!(g.player.get_status(Status::Weak), None);
        g.player.set_status(Status::Artifact, 2);
        g.step_test(EndTurnStep);
        assert_eq!(g.player.get_status(Status::Frail), Some(1));
        assert_eq!(g.player.get_status(Status::Weak), None);
        assert_eq!(g.player.get_status(Status::Artifact), Some(1));
    }

    #[test]
    fn test_warped_tongs() {
        let mut g = GameBuilder::default()
            .add_relic(RelicClass::WarpedTongs)
            .build_combat();
        g.add_card_to_draw_pile(CardClass::Strike);

        g.step_test(EndTurnStep);
        assert_eq!(g.hand[0].borrow().upgrade_count, 1);

        g.step_test(EndTurnStep);
        assert_eq!(g.hand[0].borrow().upgrade_count, 1);

        g.add_card_to_draw_pile(CardClass::SearingBlow);
        g.step_test(EndTurnStep);
        assert_eq!(g.get_hand_card(CardClass::Strike).borrow().upgrade_count, 1);
        assert_eq!(
            g.get_hand_card(CardClass::SearingBlow)
                .borrow()
                .upgrade_count,
            1
        );

        g.step_test(EndTurnStep);
        assert_eq!(g.get_hand_card(CardClass::Strike).borrow().upgrade_count, 1);
        assert_eq!(
            g.get_hand_card(CardClass::SearingBlow)
                .borrow()
                .upgrade_count,
            2
        );

        let mut found_upgraded_strike = false;
        let mut found_upgraded_defend = false;
        for _ in 0..100 {
            g.clear_all_piles();
            g.add_card_to_draw_pile_upgraded(CardClass::WildStrike);
            g.add_card_to_draw_pile(CardClass::Defend);
            g.add_card_to_draw_pile(CardClass::Strike);
            g.step_test(EndTurnStep);
            assert_eq!(
                g.hand.iter().map(|c| c.borrow().upgrade_count).sum::<i32>(),
                2
            );
            if g.get_hand_card(CardClass::Strike).borrow().upgrade_count == 1 {
                found_upgraded_strike = true;
            }
            if g.get_hand_card(CardClass::Defend).borrow().upgrade_count == 1 {
                found_upgraded_defend = true;
            }
            if found_upgraded_defend && found_upgraded_strike {
                break;
            }
        }
        assert!(found_upgraded_defend && found_upgraded_strike);
    }

    #[test]
    fn test_gambling_chip() {
        let g = GameBuilder::default()
            .add_relic(RelicClass::GamblingChip)
            .add_cards(CardClass::Strike, 10)
            .build_combat();
        assert_eq!(
            g.valid_steps(),
            vec![
                Box::new(GambleEndStep) as Box<dyn Step>,
                Box::new(GambleStep { hand_index: 0 }),
                Box::new(GambleStep { hand_index: 1 }),
                Box::new(GambleStep { hand_index: 2 }),
                Box::new(GambleStep { hand_index: 3 }),
                Box::new(GambleStep { hand_index: 4 }),
            ]
        );
    }

    #[test]
    fn test_gambling_chip_warped_tongs() {
        {
            let g = GameBuilder::default()
                .add_relic(RelicClass::GamblingChip)
                .add_relic(RelicClass::WarpedTongs)
                .add_cards(CardClass::Strike, 10)
                .build_combat();
            assert!(g.hand.iter().all(|c| c.borrow().upgrade_count == 0));
        }
        {
            let g = GameBuilder::default()
                .add_relic(RelicClass::WarpedTongs)
                .add_relic(RelicClass::GamblingChip)
                .add_cards(CardClass::Strike, 10)
                .build_combat();
            assert!(g.hand.iter().any(|c| c.borrow().upgrade_count != 0));
        }
    }

    #[test]
    fn test_ceramic_fish() {
        let mut g = GameBuilder::default()
            .add_relic(RelicClass::CeramicFish)
            .build();
        g.run_action(AddCardClassToMasterDeckAction(CardClass::Parasite));
        assert_eq!(g.gold, 9);
        g.run_action(AddCardClassToMasterDeckAction(CardClass::Strike));
        assert_eq!(g.gold, 18);
        g.add_relic(RelicClass::Omamori);
        g.run_action(AddCardClassToMasterDeckAction(CardClass::Parasite));
        assert_eq!(g.gold, 18);
    }

    #[test]
    fn test_toy_ornithopter() {
        let mut g = GameBuilder::default()
            .add_relic(RelicClass::ToyOrnithopter)
            .build_combat();
        g.player.cur_hp = 10;
        g.throw_potion(Potion::Ancient, None);
        assert_eq!(g.player.cur_hp, 15);
        g.throw_potion(Potion::Fire, Some(CreatureRef::monster(0)));
        assert_eq!(g.player.cur_hp, 20);
    }

    #[test]
    fn test_necronomicon() {
        let mut g = GameBuilder::default()
            .add_relic(RelicClass::Necronomicon)
            .build_combat();
        let hp = g.monsters[0].creature.cur_hp;
        g.play_card(CardClass::Strike, Some(CreatureRef::monster(0)));
        assert_eq!(g.monsters[0].creature.cur_hp, hp - 6);
        g.play_card(CardClass::Whirlwind, None);
        assert_eq!(g.monsters[0].creature.cur_hp, hp - 6 - 5 * 2 * 2);
        assert_eq!(g.energy, 0);

        g.step_test(EndTurnStep);
        g.energy = 4;
        g.play_card(CardClass::HandOfGreed, Some(CreatureRef::monster(0)));
        assert_eq!(g.monsters[0].creature.cur_hp, hp - 6 - 5 * 2 * 2 - 20 * 2);
        g.play_card(CardClass::HandOfGreed, Some(CreatureRef::monster(0)));
        assert_eq!(
            g.monsters[0].creature.cur_hp,
            hp - 6 - 5 * 2 * 2 - 20 * 2 - 20
        );
        assert_eq!(g.energy, 0);

        g.step_test(EndTurnStep);
        g.add_card_to_draw_pile(CardClass::HandOfGreed);
        g.play_card_upgraded(CardClass::Havoc, None);
        assert_eq!(g.energy, 3);
        assert_eq!(
            g.monsters[0].creature.cur_hp,
            hp - 6 - 5 * 2 * 2 - 20 * 2 - 20 - 20 * 2
        );
    }

    #[test]
    fn test_necronomicon_equip_unequip() {
        let mut g = GameBuilder::default()
            .add_relic(RelicClass::Necronomicon)
            .build_combat();
        assert_eq!(g.master_deck.len(), 1);
        assert_eq!(g.master_deck[0].borrow().class, CardClass::Necronomicurse);
        g.remove_relic(RelicClass::Necronomicon);
        assert_eq!(g.master_deck.len(), 0);

        g.run_action(AddCardClassToMasterDeckAction(CardClass::Necronomicurse));
        g.add_relic(RelicClass::Necronomicon);
        assert_eq!(g.master_deck.len(), 2);
        g.remove_relic(RelicClass::Necronomicon);
        assert_eq!(g.master_deck.len(), 1);

        g.master_deck.clear();
        g.add_relic(RelicClass::Omamori);
        g.add_relic(RelicClass::Necronomicon);
        assert_eq!(g.master_deck.len(), 0);
        g.remove_relic(RelicClass::Necronomicon);
        assert_eq!(g.master_deck.len(), 0);
    }

    #[test]
    fn test_enchiridion() {
        let g = GameBuilder::default()
            .add_relic(RelicClass::Enchiridion)
            .add_card(CardClass::Strike)
            .build_combat();
        assert_eq!(g.hand[0].borrow().class.ty(), CardType::Power);
        assert_eq!(g.hand[0].borrow().class.color(), CardColor::Red);
        assert_eq!(g.hand[0].borrow().get_temporary_cost(), Some(0));
    }

    #[test]
    fn test_bloody_idol() {
        let mut g = GameBuilder::default()
            .add_relic(RelicClass::BloodyIdol)
            .build_combat();
        g.player.cur_hp = 10;
        g.run_action(GainGoldAction(5));
        assert_eq!(g.player.cur_hp, 15);
    }

    #[test]
    fn test_mercury_hourglass() {
        let mut g = GameBuilder::default()
            .add_relic(RelicClass::MercuryHourglass)
            .build_combat_with_monsters(NoopMonster::new(), NoopMonster::new());
        assert_eq!(
            g.monsters[0].creature.cur_hp,
            g.monsters[0].creature.max_hp - 3
        );
        assert_eq!(
            g.monsters[1].creature.cur_hp,
            g.monsters[1].creature.max_hp - 3
        );
        g.step_test(EndTurnStep);
        assert_eq!(
            g.monsters[0].creature.cur_hp,
            g.monsters[0].creature.max_hp - 6
        );
        assert_eq!(
            g.monsters[1].creature.cur_hp,
            g.monsters[1].creature.max_hp - 6
        );
    }

    #[test]
    fn test_stone_calendar() {
        let mut g = GameBuilder::default()
            .add_relic(RelicClass::StoneCalendar)
            .build_combat_with_monsters(NoopMonster::new(), AttackMonster::new(1));
        for _ in 0..6 {
            assert_eq!(g.monsters[0].creature.cur_hp, g.monsters[0].creature.max_hp);
            assert_eq!(g.monsters[1].creature.cur_hp, g.monsters[1].creature.max_hp);
            g.step_test(EndTurnStep);
        }
        assert_eq!(g.monsters[0].creature.cur_hp, g.monsters[0].creature.max_hp);
        assert_eq!(g.monsters[1].creature.cur_hp, g.monsters[1].creature.max_hp);
        g.monsters[1].creature.cur_hp = 51;
        let hp = g.player.cur_hp;
        g.step_test(EndTurnStep);
        assert_eq!(g.player.cur_hp, hp);
        assert_eq!(
            g.monsters[0].creature.cur_hp,
            g.monsters[0].creature.max_hp - 52
        );
    }

    #[test]
    fn test_strange_spoon() {
        let mut g = GameBuilder::default()
            .add_relic(RelicClass::StrangeSpoon)
            .build_combat();
        let mut exhausted = false;
        let mut discarded = false;
        g.play_card(CardClass::FeelNoPain, None);
        for _ in 0..100 {
            g.clear_all_piles();
            g.player.block = 0;
            g.play_card(CardClass::BandageUp, None);
            assert_eq!(g.discard_pile.len() + g.exhaust_pile.len(), 1);
            if !g.discard_pile.is_empty() {
                discarded = true;
                assert_eq!(g.player.block, 0);
            } else {
                exhausted = true;
                assert_ne!(g.player.block, 0);
            }
            if exhausted && discarded {
                break;
            }
        }
        assert!(exhausted && discarded);
    }

    #[test]
    fn test_brimstone() {
        let mut g = GameBuilder::default()
            .add_relic(RelicClass::Brimstone)
            .build_combat();
        assert_eq!(g.player.get_status(Status::Strength), Some(2));
        assert_eq!(g.monsters[0].creature.get_status(Status::Strength), Some(1));
        g.step_test(EndTurnStep);
        assert_eq!(g.player.get_status(Status::Strength), Some(4));
        assert_eq!(g.monsters[0].creature.get_status(Status::Strength), Some(2));
    }

    #[test]
    fn test_mummified_hand() {
        let mut g = GameBuilder::default()
            .add_relic(RelicClass::MummifiedHand)
            .build_combat();
        g.play_card(CardClass::Berserk, None);

        g.add_card_to_hand(CardClass::Strike);
        g.play_card(CardClass::Berserk, None);
        assert_eq!(g.hand[0].borrow().get_temporary_cost(), Some(0));

        for _ in 0..10 {
            g.clear_all_piles();
            g.add_card_to_hand(CardClass::Strike);
            g.hand[0].borrow_mut().set_free_to_play_once();
            g.add_card_to_hand(CardClass::Defend);
            g.play_card(CardClass::Berserk, None);
            assert_eq!(g.hand[1].borrow().get_temporary_cost(), Some(0));

            g.clear_all_piles();
            g.add_card_to_hand(CardClass::Strike);
            g.hand[0].borrow_mut().set_temporary_cost(0);
            g.add_card_to_hand(CardClass::Defend);
            g.play_card(CardClass::Berserk, None);
            assert_eq!(g.hand[1].borrow().get_temporary_cost(), Some(0));

            g.clear_all_piles();
            g.add_card_to_hand(CardClass::Strike);
            g.hand[0].borrow_mut().set_cost(0, None);
            g.add_card_to_hand(CardClass::Defend);
            g.play_card(CardClass::Berserk, None);
            assert_eq!(g.hand[1].borrow().get_temporary_cost(), Some(0));
        }

        let mut discount_0 = false;
        let mut discount_1 = false;
        for _ in 0..100 {
            g.clear_all_piles();
            g.add_card_to_hand(CardClass::Strike);
            g.add_card_to_hand(CardClass::Defend);
            g.play_card(CardClass::Berserk, None);
            assert_ne!(
                g.hand[0].borrow().get_temporary_cost(),
                g.hand[1].borrow().get_temporary_cost()
            );
            if g.hand[0].borrow().get_temporary_cost() == Some(0) {
                discount_0 = true;
            }
            if g.hand[1].borrow().get_temporary_cost() == Some(0) {
                discount_1 = true;
            }
            if discount_0 && discount_1 {
                break;
            }
        }
        assert!(discount_0 && discount_1);
    }

    #[test]
    fn test_centennial_puzzle() {
        {
            let mut g = GameBuilder::default()
                .add_relic(RelicClass::CentennialPuzzle)
                .build_combat();
            g.add_cards_to_draw_pile(CardClass::Strike, 10);
            g.play_card(CardClass::Bloodletting, None);
            assert_eq!(g.hand.len(), 3);
            g.play_card(CardClass::Bloodletting, None);
            assert_eq!(g.hand.len(), 3);
        }
        {
            let mut g = GameBuilder::default()
                .add_cards(CardClass::Strike, 10)
                .add_relic(RelicClass::CentennialPuzzle)
                .build_combat_with_monster(AttackMonster::new(1));
            g.play_card(CardClass::Thunderclap, None);
            g.step_test(EndTurnStep);
            assert_eq!(g.hand.len(), 8);
            g.step_test(EndTurnStep);
            assert_eq!(g.hand.len(), 5);
        }
    }

    #[test]
    fn test_self_forming_clay() {
        let mut g = GameBuilder::default()
            .add_relic(RelicClass::SelfFormingClay)
            .build_combat_with_monster(AttackMonster::with_attack_count(1, 2));
        g.play_card(CardClass::Thunderclap, None);
        assert_eq!(
            g.monsters[0].creature.get_status(Status::NextTurnBlock),
            None
        );
        g.play_card(CardClass::Bloodletting, None);
        assert_eq!(g.player.get_status(Status::NextTurnBlock), Some(3));
        g.step_test(EndTurnStep);
        g.step_test(EndTurnStep);
        g.step_test(EndTurnStep);
        assert_eq!(g.player.block, 6);
    }

    #[test]
    fn test_mark_of_the_bloom() {
        let mut g = GameBuilder::default()
            .add_relic(RelicClass::MarkOfTheBloom)
            .add_relic(RelicClass::LizardTail)
            .build_combat_with_monster(AttackMonster::new(999));
        g.add_potion(Potion::Fairy);
        g.player.cur_hp = 10;
        g.play_card(CardClass::Reaper, None);
        g.play_card(CardClass::BandageUp, None);
        assert_eq!(g.player.cur_hp, 10);
        g.step_test(EndTurnStep);
        assert_matches!(g.status, GameStatus::Defeat);
    }

    #[test]
    fn test_lizard_tail() {
        {
            let mut g = GameBuilder::default()
                .add_relic(RelicClass::LizardTail)
                .build_combat_with_monster(AttackMonster::new(1000));
            assert_eq!(g.get_relic_value(RelicClass::LizardTail), Some(1));

            g.step_test(EndTurnStep);
            assert!(g.player.is_actionable());
            assert_eq!(g.player.cur_hp, (g.player.max_hp as f32 * 0.5) as i32);
            assert_eq!(g.get_relic_value(RelicClass::LizardTail), Some(0));
        }
        {
            let mut g = GameBuilder::default()
                .add_relic(RelicClass::LizardTail)
                .build_combat_with_monster(AttackMonster::new(1000));

            g.player.decrease_max_hp(g.player.max_hp - 1);
            g.step_test(EndTurnStep);
            assert_eq!(g.player.cur_hp, 1);
        }
        {
            let mut g = GameBuilder::default()
                .add_relic(RelicClass::LizardTail)
                .build_combat_with_monster(AttackMonster::with_attack_count(1000, 2));
            g.step_test(EndTurnStep);
            assert!(!g.player.is_actionable());
        }
        {
            // test that fairy is used before lizard tail
            let mut g = GameBuilder::default()
                .add_relic(RelicClass::LizardTail)
                .build_combat_with_monster(AttackMonster::new(1000));
            g.add_potion(Potion::Fairy);
            g.step_test(EndTurnStep);
            assert_eq!(g.get_relic_value(RelicClass::LizardTail), Some(1));
            assert!(g.potions.iter().all(|p| p.is_none()));
        }
    }

    #[test]
    fn test_champion_belt() {
        {
            let mut g = GameBuilder::default()
                .add_relic(RelicClass::ChampionBelt)
                .build_combat();
            g.play_card(CardClass::Bash, Some(CreatureRef::monster(0)));
            assert_eq!(g.monsters[0].creature.get_status(Status::Weak), Some(1));
            g.play_card(CardClass::Thunderclap, None);
            assert_eq!(g.monsters[0].creature.get_status(Status::Weak), Some(2));
        }
        {
            let mut g = GameBuilder::default()
                .add_relic(RelicClass::ChampionBelt)
                .build_combat();
            g.monsters[0].creature.set_status(Status::Artifact, 1);
            g.play_card(CardClass::Bash, Some(CreatureRef::monster(0)));
            assert_eq!(g.monsters[0].creature.get_status(Status::Weak), None);
            assert_eq!(g.monsters[0].creature.get_status(Status::Artifact), None);
        }
    }

    #[test]
    fn test_orichalcum() {
        let mut g = GameBuilder::default()
            .add_relic(RelicClass::Orichalcum)
            .build_combat_with_monster(AttackMonster::new(10));
        let hp = g.player.cur_hp;
        g.step_test(EndTurnStep);
        assert_eq!(g.player.cur_hp, hp - 4);

        g.player.block = 1;
        g.step_test(EndTurnStep);
        assert_eq!(g.player.cur_hp, hp - 4 - 9);

        g.player.set_status(Status::FeelNoPain, 1);
        g.add_card_to_hand(CardClass::Dazed);
        g.step_test(EndTurnStep);
        assert_eq!(g.player.cur_hp, hp - 4 - 9 - 3);

        g.player.set_status(Status::PlatedArmor, 1);
        g.player.set_status(Status::Metallicize, 2);
        g.step_test(EndTurnStep);
        assert_eq!(g.player.cur_hp, hp - 4 - 9 - 3 - 1);
    }

    #[test]
    fn test_torii() {
        {
            let mut g = GameBuilder::default()
                .add_relic(RelicClass::Torii)
                .build_combat_with_monster(AttackMonster::new(5));

            let hp = g.player.cur_hp;
            g.step_test(EndTurnStep);
            assert_eq!(g.player.cur_hp, hp - 1);

            g.monsters[0].creature.set_status(Status::Strength, 1);
            g.step_test(EndTurnStep);
            assert_eq!(g.player.cur_hp, hp - 1 - 6);
        }
        {
            let mut g = GameBuilder::default()
                .add_relic(RelicClass::Torii)
                .build_combat();
            g.add_card_to_hand(CardClass::Burn);
            let hp = g.player.cur_hp;
            g.step_test(EndTurnStep);
            assert_eq!(g.player.cur_hp, hp - 2);
        }
        {
            let mut g = GameBuilder::default()
                .add_relic(RelicClass::Torii)
                .build_combat_with_monster(AttackMonster::with_attack_count(4, 2));
            let hp = g.player.cur_hp;
            g.player.block = 3;
            g.step_test(EndTurnStep);
            assert_eq!(g.player.cur_hp, hp - 2);
        }
    }

    #[test]
    fn test_torii_tungsten_rod() {
        let mut g = GameBuilder::default()
            .add_relic(RelicClass::Torii)
            .add_relic(RelicClass::TungstenRod)
            .build_combat_with_monster(AttackMonster::new(5));
        let hp = g.player.cur_hp;
        g.step_test(EndTurnStep);
        assert_eq!(g.player.cur_hp, hp);
    }

    #[test]
    fn test_meat_on_the_bone() {
        for (cur, max, expected) in [(5, 10, 10), (5, 50, 17), (6, 10, 6)] {
            let mut g = GameBuilder::default()
                .add_relic(RelicClass::MeatOnTheBone)
                .build_combat();
            g.player.cur_hp = cur;
            g.player.max_hp = max;
            g.play_card(CardClass::DebugKillAll, None);
            assert_eq!(g.player.cur_hp, expected);
        }
    }

    #[test]
    fn test_meat_on_the_bone_burning_blood() {
        let mut g = GameBuilder::default()
            .add_relic(RelicClass::BurningBlood)
            .add_relic(RelicClass::MeatOnTheBone)
            .build_combat();
        g.player.cur_hp = 49;
        g.player.max_hp = 100;
        g.play_card(CardClass::DebugKillAll, None);
        assert_eq!(g.player.cur_hp, 49 + 12 + 6);
    }

    #[test]
    fn test_red_skull() {
        let mut g = GameBuilder::default()
            .add_relic(RelicClass::RedSkull)
            .build_with_rooms(&[RoomType::Monster, RoomType::Monster]);
        g.roll_noop_monsters = true;
        g.step_test(AscendStep::new(0, 0));
        g.player.cur_hp = g.player.max_hp / 2 + 1;
        assert_eq!(g.player.get_status(Status::Strength), None);
        g.play_card(CardClass::Bloodletting, None);
        assert_eq!(g.player.get_status(Status::Strength), Some(3));
        g.play_card(CardClass::BandageUp, None);
        assert_eq!(g.player.get_status(Status::Strength), None);
        g.play_card(CardClass::Offering, None);
        assert_eq!(g.player.get_status(Status::Strength), Some(3));

        // start new combat bloodied
        g.play_card(CardClass::DebugKillAll, None);
        g.step_test(RewardExitStep);
        g.step_test(AscendStep::new(0, 1));
        assert!(g.player.is_bloodied());
        assert_eq!(g.monsters[0].creature.cur_hp, g.monsters[0].creature.max_hp);
        assert_eq!(g.player.get_status(Status::Strength), Some(3));
        // test that artifact blocks strength down
        g.play_card(CardClass::Panacea, None);
        g.play_card_upgraded(CardClass::BandageUp, None);
        assert!(!g.player.is_bloodied());
        assert_eq!(g.player.get_status(Status::Strength), Some(3));
        assert_eq!(g.player.get_status(Status::Artifact), None)
    }

    #[test]
    fn test_hand_drill() {
        let mut g = GameBuilder::default()
            .add_relic(RelicClass::HandDrill)
            .build_combat();
        g.monsters[0].creature.block = 10;
        g.play_card(CardClass::SwiftStrike, Some(CreatureRef::monster(0)));
        assert_eq!(g.monsters[0].creature.get_status(Status::Vulnerable), None);
        g.play_card(CardClass::SwiftStrike, Some(CreatureRef::monster(0)));
        assert_eq!(
            g.monsters[0].creature.get_status(Status::Vulnerable),
            Some(2)
        );

        g.monsters[0].creature.block = 10;
        g.player.set_status(Status::CombustDamage, 12);
        g.step_test(EndTurnStep);
        assert_eq!(
            g.monsters[0].creature.get_status(Status::Vulnerable),
            Some(3)
        );
    }

    #[test]
    fn test_hand_drill_dropkick() {
        let mut g = GameBuilder::default()
            .add_relic(RelicClass::HandDrill)
            .build_combat();
        g.monsters[0].creature.block = 1;
        g.play_card(CardClass::Dropkick, Some(CreatureRef::monster(0)));
        assert_eq!(g.energy, 2);
    }

    #[test]
    fn test_charons_ashes() {
        let mut g = GameBuilder::default()
            .add_relic(RelicClass::CharonsAshes)
            .build_combat_with_monsters(NoopMonster::with_hp(50), NoopMonster::with_hp(50));
        g.add_cards_to_hand(CardClass::Dazed, 2);
        g.monsters[1].creature.block = 1;
        g.step_test(EndTurnStep);
        assert_eq!(g.monsters[0].creature.cur_hp, 44);
        assert_eq!(g.monsters[1].creature.cur_hp, 45);
    }

    #[test]
    fn test_runic_cube() {
        let mut g = GameBuilder::default()
            .add_relic(RelicClass::RunicCube)
            .add_cards(CardClass::Strike, 10)
            .build_combat_with_monster(AttackMonster::with_attack_count(1, 4));
        assert_eq!(g.hand.len(), 5);
        g.play_card(CardClass::Bloodletting, None);
        assert_eq!(g.hand.len(), 6);
        g.step_test(EndTurnStep);
        assert_eq!(g.hand.len(), 9);
    }

    #[test]
    fn test_toolbox() {
        for _ in 0..20 {
            let mut g = GameBuilder::default()
                .add_relic(RelicClass::Toolbox)
                .add_card(CardClass::Strike)
                .build_combat();
            assert_eq!(g.valid_steps().len(), 3);
            g.step(0);
            if g.hand[0].borrow().class != CardClass::Transmutation {
                assert_eq!(g.hand[0].borrow().get_temporary_cost(), None);
            }
            assert_eq!(g.hand[0].borrow().class.color(), CardColor::Colorless);
        }
    }

    #[test]
    fn test_runic_pyramid() {
        let mut g = GameBuilder::default()
            .add_relic(RelicClass::RunicPyramid)
            .add_cards(CardClass::Strike, 20)
            .build_combat();
        assert_eq!(g.hand.len(), 5);
        g.add_card_to_hand(CardClass::Apparition);
        g.step_test(EndTurnStep);
        assert_eq!(g.hand.len(), 10);
        g.hand
            .iter()
            .for_each(|c| assert_eq!(c.borrow().class, CardClass::Strike));
    }

    #[test]
    fn test_neows_lament() {
        let mut g = GameBuilder::default()
            .add_relic(RelicClass::NeowsLament)
            .build_with_rooms(&[
                RoomType::Monster,
                RoomType::Monster,
                RoomType::Monster,
                RoomType::Monster,
            ]);

        g.step_test(AscendStep::new(0, 0));

        assert_eq!(g.get_relic_value(RelicClass::NeowsLament), Some(2));
        assert_eq!(g.monsters[0].creature.cur_hp, 1);
        g.play_card(CardClass::Thunderclap, None);

        g.step_test(RewardExitStep);
        g.step_test(AscendStep::new(0, 1));

        assert_eq!(g.get_relic_value(RelicClass::NeowsLament), Some(1));
        assert_eq!(g.monsters[0].creature.cur_hp, 1);
        g.play_card(CardClass::Thunderclap, None);

        g.step_test(RewardExitStep);
        g.step_test(AscendStep::new(0, 2));

        assert_eq!(g.get_relic_value(RelicClass::NeowsLament), Some(0));
        assert_eq!(g.monsters[0].creature.cur_hp, 1);
        g.play_card(CardClass::Thunderclap, None);

        g.step_test(RewardExitStep);
        g.step_test(AscendStep::new(0, 3));

        assert_eq!(g.get_relic_value(RelicClass::NeowsLament), Some(0));
        assert_eq!(g.monsters[0].creature.cur_hp, g.monsters[0].creature.max_hp);
    }

    #[test]
    fn test_neows_lament_mercury_hourglass() {
        let g = GameBuilder::default()
            .add_relic(RelicClass::NeowsLament)
            .add_relic(RelicClass::MercuryHourglass)
            .build_combat();
        assert!(g.monsters.is_empty());
    }

    #[test]
    fn test_nilrys_codex_1() {
        let mut g = GameBuilder::default()
            .add_relic(RelicClass::NilrysCodex)
            .build_combat();

        let mut found_twin = false;
        for _ in 0..10000 {
            g.step_test(EndTurnStep);
            let valid_steps = g.valid_steps();
            assert_ne!(valid_steps[1], valid_steps[2]);
            assert_ne!(valid_steps[1], valid_steps[3]);
            assert_ne!(valid_steps[2], valid_steps[3]);
            g.step(1);
            if g.hand[0].borrow().class == CardClass::TwinStrike {
                found_twin = true;
                break;
            }
            g.clear_all_piles();
        }
        assert!(found_twin);
    }

    #[test]
    fn test_nilrys_codex_2() {
        let mut g = GameBuilder::default()
            .add_relic(RelicClass::NilrysCodex)
            .add_relic(RelicClass::RunicPyramid)
            .add_cards(CardClass::Strike, 20)
            .build_combat();
        g.step_test(EndTurnStep);

        assert_eq!(g.valid_steps().len(), 4);
        assert_eq!(g.valid_steps()[0], Box::new(ContinueStep) as Box<dyn Step>);

        g.step_test(ContinueStep);

        for i in 1..10 {
            assert_eq!(g.hand.len(), 10);
            g.step_test(EndTurnStep);
            g.step_test_no_check_valid(ChooseCardToShuffleIntoDrawStep {
                class: CardClass::BloodForBlood,
            });
            assert_eq!(
                g.draw_pile
                    .get_all()
                    .into_iter()
                    .filter(|c| c.borrow().class == CardClass::BloodForBlood)
                    .count(),
                i
            );
            assert_eq!(
                g.draw_pile.top(&mut g.rng).borrow().class,
                CardClass::Strike
            );
            for c in g.draw_pile.get_all() {
                assert_eq!(c.borrow().upgrade_count, 0);
            }
        }
    }

    #[test]
    fn test_dead_branch() {
        let mut g = GameBuilder::default()
            .add_relic(RelicClass::DeadBranch)
            .build_combat();
        g.play_card(CardClass::Offering, None);
        assert_eq!(g.hand.len(), 1);
        assert_eq!(g.hand[0].borrow().class.color(), CardColor::Red);

        g.clear_all_piles();
        g.add_card_to_hand(CardClass::Apparition);
        g.step_test(EndTurnStep);
        assert_eq!(g.hand.len(), 1);
        assert_eq!(g.hand[0].borrow().class.color(), CardColor::Red);
    }

    #[test]
    fn test_mark_of_pain() {
        let mut g = GameBuilder::default()
            .add_relic(RelicClass::MarkOfPain)
            .build_combat();
        assert_eq!(g.hand.len(), 0);
        g.step_test(EndTurnStep);
        assert_eq!(g.hand.len(), 2);
        for c in &g.hand {
            assert_eq!(c.borrow().class, CardClass::Wound);
        }
    }

    #[test]
    fn test_velvet_choker() {
        let mut g = GameBuilder::default()
            .add_relic(RelicClass::VelvetChoker)
            .add_card(CardClass::SwiftStrike)
            .build_combat();
        assert_eq!(g.energy, 4);
        g.play_card(CardClass::SwiftStrike, Some(CreatureRef::monster(0)));
        g.play_card(CardClass::SwiftStrike, Some(CreatureRef::monster(0)));
        g.play_card(CardClass::SwiftStrike, Some(CreatureRef::monster(0)));
        g.play_card(CardClass::SwiftStrike, Some(CreatureRef::monster(0)));
        g.play_card(CardClass::SwiftStrike, Some(CreatureRef::monster(0)));
        let num_steps = g.valid_steps().len();
        g.play_card(CardClass::SwiftStrike, Some(CreatureRef::monster(0)));
        assert_eq!(num_steps - 1, g.valid_steps().len());
        g.step_test(EndTurnStep);
        assert_eq!(g.num_cards_played_this_turn, 0);
        g.throw_potion(Potion::Chaos, None);
        g.throw_potion(Potion::Chaos, None);
        g.throw_potion(Potion::Chaos, None);
        assert_eq!(g.num_cards_played_this_turn, 6);
    }

    #[test]
    fn test_sozu() {
        let mut g = GameBuilder::default()
            .add_relic(RelicClass::Sozu)
            .build_combat();
        assert_eq!(g.energy, 4);
        g.add_potion(Potion::Ancient);
        assert_eq!(g.potions, vec![None, None]);
    }

    #[test]
    fn test_regal_pillow() {
        let mut g = GameBuilder::default()
            .add_relic(RelicClass::RegalPillow)
            .build_campfire();
        g.player.cur_hp = 10;
        g.player.max_hp = 100;
        g.step_test(CampfireRestStep);
        assert_eq!(g.player.cur_hp, 10 + 30 + 15)
    }

    #[test]
    fn test_meal_ticket() {
        let mut g = GameBuilder::default()
            .add_relic(RelicClass::MealTicket)
            .build_with_rooms(&[RoomType::Shop, RoomType::Campfire]);
        g.player.cur_hp = 10;
        g.step_test(AscendStep::new(0, 0));
        assert_eq!(g.player.cur_hp, 25);
    }

    #[test]
    fn test_preserved_insect() {
        {
            let mut g = GameBuilder::default()
                .add_relic(RelicClass::PreservedInsect)
                .build_with_rooms(&[RoomType::Monster]);
            g.step_test(AscendStep::new(0, 0));
            assert!(!g.monsters.is_empty());
            for m in &g.monsters {
                assert_eq!(m.creature.cur_hp, m.creature.max_hp);
            }
        }
        {
            let mut g = GameBuilder::default()
                .add_relic(RelicClass::PreservedInsect)
                .build_with_rooms(&[RoomType::Elite]);
            g.step_test(AscendStep::new(0, 0));
            assert!(!g.monsters.is_empty());
            for m in &g.monsters {
                assert_eq!(m.creature.cur_hp, (m.creature.max_hp as f32 * 0.75) as i32);
            }
        }
    }

    #[test]
    fn test_sling_of_courage() {
        {
            let mut g = GameBuilder::default()
                .add_relic(RelicClass::SlingOfCourage)
                .build_with_rooms(&[RoomType::Monster]);
            g.step_test(AscendStep::new(0, 0));
            assert_eq!(g.player.get_status(Status::Strength), None);
        }
        {
            let mut g = GameBuilder::default()
                .add_relic(RelicClass::SlingOfCourage)
                .build_with_rooms(&[RoomType::Elite]);
            g.step_test(AscendStep::new(0, 0));
            assert_eq!(g.player.get_status(Status::Strength), Some(2));
        }
    }

    #[test]
    fn test_unceasing_top() {
        {
            let mut g = GameBuilder::default()
                .add_relic(RelicClass::UnceasingTop)
                .add_card(CardClass::GoodInstincts)
                .build_combat();
            g.step_test(PlayCardStep {
                hand_index: 0,
                target: None,
            });
            assert_eq!(g.hand.len(), 1);
            assert!(g.draw_pile.is_empty());
            assert!(g.discard_pile.is_empty());
        }
        {
            let mut g = GameBuilder::default()
                .add_relic(RelicClass::UnceasingTop)
                .add_card(CardClass::BattleTrance)
                .build_combat();
            g.step_test(PlayCardStep {
                hand_index: 0,
                target: None,
            });
            assert!(g.hand.is_empty());
            assert!(g.draw_pile.is_empty());
            assert!(!g.discard_pile.is_empty());
        }
        {
            let mut g = GameBuilder::default()
                .add_relic(RelicClass::UnceasingTop)
                .add_card(CardClass::Exhume)
                .build_combat();
            g.step_test(PlayCardStep {
                hand_index: 0,
                target: None,
            });
            assert!(g.hand.is_empty());
            assert!(g.draw_pile.is_empty());
            assert!(g.discard_pile.is_empty());
        }
    }

    #[test]
    fn test_ssserpent_head() {
        let mut g = GameBuilder::default()
            .add_relic(RelicClass::SsserpentHead)
            .build_with_rooms(&[RoomType::Monster, RoomType::Event]);
        g.step_test(AscendStep::new(0, 0));
        g.play_card(CardClass::DebugKillAll, None);
        g.step_test(RewardExitStep);
        assert_eq!(g.gold, 0);
        g.step_test(AscendStep::new(0, 1));
        assert_eq!(g.gold, 50);
    }

    #[test]
    fn test_dollys_mirror() {
        {
            let mut g = GameBuilder::default()
                .add_card(CardClass::Strike)
                .add_card_upgraded(CardClass::Defend)
                .build();
            g.run_action(GainRelicAction(RelicClass::DollysMirror));
            g.step_test(DuplicateCardInMasterStep { master_index: 1 });
            assert_eq!(g.master_deck.len(), 3);
        }
        {
            let mut g = GameBuilder::default()
                .add_card(CardClass::RitualDagger)
                .build();
            g.master_deck[0].borrow_mut().base_increase = 5;
            g.run_action(GainRelicAction(RelicClass::DollysMirror));
            g.step_test(DuplicateCardInMasterStep { master_index: 0 });
            assert_ne!(g.master_deck[0].borrow().id, g.master_deck[1].borrow().id);
            assert_eq!(
                g.master_deck[0].borrow().base_increase,
                g.master_deck[1].borrow().base_increase
            );
        }
        {
            let mut g = GameBuilder::default().add_card(CardClass::Strike).build();
            g.master_deck[0].borrow_mut().is_bottled = true;
            g.run_action(GainRelicAction(RelicClass::DollysMirror));
            g.step_test(DuplicateCardInMasterStep { master_index: 0 });
            assert!(g.master_deck[0].borrow().is_bottled);
            assert!(!g.master_deck[1].borrow().is_bottled);
        }
    }

    #[test]
    fn test_eternal_feather() {
        for (num_cards, heal) in [(4, 0), (5, 3), (24, 12)] {
            let mut g = GameBuilder::default()
                .add_cards(CardClass::Strike, num_cards)
                .add_relic(RelicClass::EternalFeather)
                .build_with_rooms(&[RoomType::Campfire]);
            g.player.cur_hp = 20;
            g.step_test(AscendStep::new(0, 0));
            assert_eq!(g.player.cur_hp, 20 + heal);
        }
    }

    #[test]
    fn test_molten_egg() {
        let mut g = GameBuilder::default()
            .add_relic(RelicClass::MoltenEgg)
            .build();
        g.run_action(AddCardClassToMasterDeckAction(CardClass::Anger));
        assert_eq!(g.master_deck[0].borrow().upgrade_count, 1);
        g.run_action(AddCardClassToMasterDeckAction(CardClass::Defend));
        assert_eq!(g.master_deck[1].borrow().upgrade_count, 0);
        let c = g.new_card_upgraded(CardClass::SearingBlow);
        g.run_action(AddCardToMasterDeckAction(c));
        assert_eq!(g.master_deck[2].borrow().upgrade_count, 1);
    }

    #[test]
    fn test_toxic_egg() {
        let mut g = GameBuilder::default()
            .add_relic(RelicClass::ToxicEgg)
            .build();
        g.run_action(AddCardClassToMasterDeckAction(CardClass::Defend));
        assert_eq!(g.master_deck[0].borrow().upgrade_count, 1);
        g.run_action(AddCardClassToMasterDeckAction(CardClass::DemonForm));
        assert_eq!(g.master_deck[1].borrow().upgrade_count, 0);
    }

    #[test]
    fn test_frozen_egg() {
        let mut g = GameBuilder::default()
            .add_relic(RelicClass::FrozenEgg)
            .build();
        g.run_action(AddCardClassToMasterDeckAction(CardClass::Defend));
        assert_eq!(g.master_deck[0].borrow().upgrade_count, 0);
        g.run_action(AddCardClassToMasterDeckAction(CardClass::DemonForm));
        assert_eq!(g.master_deck[1].borrow().upgrade_count, 1);
    }

    #[test]
    fn test_slavers_collar() {
        let mut g = GameBuilder::default()
            .add_relic(RelicClass::SlaversCollar)
            .build_with_rooms(&[RoomType::Monster, RoomType::Elite, RoomType::Boss]);
        g.step_test(AscendStep::new(0, 0));
        assert_eq!(g.energy, 3);
        g.play_card(CardClass::DebugKillAll, None);
        g.step_test(RewardExitStep);
        g.step_test(AscendStep::new(0, 1));
        assert_eq!(g.energy, 4);
        g.play_card(CardClass::DebugKillAll, None);
        g.step_test(RewardExitStep);
        g.step_test(AscendStep::new(0, 2));
        assert_eq!(g.energy, 4);
    }

    #[test]
    fn test_pantograph() {
        let mut g = GameBuilder::default()
            .add_relic(RelicClass::Pantograph)
            .build_with_rooms(&[RoomType::Elite, RoomType::Boss]);
        g.player.cur_hp = 10;
        g.step_test(AscendStep::new(0, 0));
        assert_eq!(g.player.cur_hp, 10);
        g.play_card(CardClass::DebugKillAll, None);
        g.step_test(RewardExitStep);
        g.step_test(AscendStep::new(0, 1));
        assert_eq!(g.player.cur_hp, 35);
    }

    #[test]
    fn test_magic_flower() {
        let mut g = GameBuilder::default()
            .add_relic(RelicClass::MagicFlower)
            .build_with_rooms(&[RoomType::Monster]);
        g.player.cur_hp = 10;
        g.player.max_hp = 100;
        g.throw_potion(Potion::Blood, None);
        assert_eq!(g.player.cur_hp, 30);
        g.step_test(AscendStep::new(0, 0));
        g.throw_potion(Potion::Blood, None);
        assert_eq!(g.player.cur_hp, 60);
        g.play_card_upgraded(CardClass::Bite, Some(CreatureRef::monster(0)));
        assert_eq!(g.player.cur_hp, 65);
    }

    #[test]
    fn test_empty_cage() {
        let mut g = GameBuilder::default()
            .add_card(CardClass::Strike)
            .add_card(CardClass::Defend)
            .add_card(CardClass::Bash)
            .build();
        g.run_action(GainRelicAction(RelicClass::EmptyCage));
        g.step_test(ChooseRemoveFromMasterStep {
            master_index: 2,
            num_cards_remaining: 2,
        });
        g.step_test(ChooseRemoveFromMasterStep {
            master_index: 0,
            num_cards_remaining: 1,
        });
        assert_eq!(g.master_deck.len(), 1);
        assert_eq!(g.master_deck[0].borrow().class, CardClass::Defend);
    }

    #[test]
    fn test_empty_cage_few_cards() {
        {
            let mut g = GameBuilder::default()
                .add_card(CardClass::Strike)
                .add_card(CardClass::Defend)
                .build();
            g.run_action(GainRelicAction(RelicClass::EmptyCage));
            assert!(g.master_deck.is_empty());
        }
        {
            let mut g = GameBuilder::default().add_card(CardClass::Parasite).build();
            g.player.max_hp = 50;
            g.run_action(GainRelicAction(RelicClass::EmptyCage));
            assert!(g.master_deck.is_empty());
            assert_eq!(g.player.max_hp, 47);
        }
    }

    #[test]
    fn test_orange_pellets() {
        {
            let mut g = GameBuilder::default()
                .add_relic(RelicClass::OrangePellets)
                .build_combat();
            g.run_action(GainStatusAction {
                status: Status::Weak,
                amount: 1,
                target: CreatureRef::player(),
            });
            g.run_action(GainStatusAction {
                status: Status::Dexterity,
                amount: 1,
                target: CreatureRef::player(),
            });
            g.run_action(GainStatusAction {
                status: Status::Strength,
                amount: -1,
                target: CreatureRef::player(),
            });

            g.play_card(CardClass::SwiftStrike, Some(CreatureRef::monster(0)));
            g.play_card(CardClass::FeelNoPain, None);
            g.play_card(CardClass::SwiftStrike, Some(CreatureRef::monster(0)));
            assert_eq!(g.player.get_status(Status::Weak), Some(1));
            assert_eq!(g.player.get_status(Status::Dexterity), Some(1));
            assert_eq!(g.player.get_status(Status::Strength), Some(-1));

            g.play_card(CardClass::GoodInstincts, None);
            assert_eq!(g.player.get_status(Status::Weak), None);
            assert_eq!(g.player.get_status(Status::Dexterity), Some(1));
            assert_eq!(g.player.get_status(Status::Strength), None);
        }
        {
            let mut g = GameBuilder::default()
                .add_relic(RelicClass::OrangePellets)
                .build_combat();
            g.run_action(GainStatusAction {
                status: Status::Confusion,
                amount: 1,
                target: CreatureRef::player(),
            });
            g.run_action(GainStatusAction {
                status: Status::Dexterity,
                amount: 1,
                target: CreatureRef::player(),
            });
            g.run_action(GainStatusAction {
                status: Status::Strength,
                amount: -1,
                target: CreatureRef::player(),
            });

            g.play_card(CardClass::SwiftStrike, Some(CreatureRef::monster(0)));
            g.play_card(CardClass::FeelNoPain, None);
            g.play_card(CardClass::SwiftStrike, Some(CreatureRef::monster(0)));
            assert_eq!(g.player.get_status(Status::Confusion), Some(1));
            assert_eq!(g.player.get_status(Status::Dexterity), Some(1));
            assert_eq!(g.player.get_status(Status::Strength), Some(-1));

            g.step_test(EndTurnStep);
            g.play_card(CardClass::GoodInstincts, None);
            assert_eq!(g.player.get_status(Status::Confusion), Some(1));
            assert_eq!(g.player.get_status(Status::Dexterity), Some(1));
            assert_eq!(g.player.get_status(Status::Strength), Some(-1));
        }
    }

    #[test]
    fn test_pandoras_box() {
        let mut g = GameBuilder::default()
            .add_cards(CardClass::Strike, 2)
            .add_cards(CardClass::Defend, 2)
            .add_card(CardClass::Bash)
            .build();
        g.run_action(GainRelicAction(RelicClass::PandorasBox));
        assert_eq!(g.master_deck.len(), 5);
        assert_eq!(g.master_deck[0].borrow().class, CardClass::Bash);
        for i in 1..5 {
            assert_ne!(g.master_deck[i].borrow().class.rarity(), CardRarity::Basic);
        }
    }

    #[test]
    fn test_astrolabe() {
        {
            let mut g = GameBuilder::default()
                // molten egg to test searing blow is +1 not +2
                .add_relic(RelicClass::MoltenEgg)
                .add_cards(CardClass::Strike, 2)
                .add_card(CardClass::Bash)
                .add_card(CardClass::AscendersBane)
                .build();
            g.run_action(GainRelicAction(RelicClass::Astrolabe));
            for c in g.master_deck.iter().skip(1) {
                assert_ne!(c.borrow().class.rarity(), CardRarity::Basic);
                assert_eq!(c.borrow().upgrade_count, 1);
            }
        }
        {
            let mut g = GameBuilder::default()
                // molten egg to test searing blow is +1 not +2
                .add_relic(RelicClass::MoltenEgg)
                .add_cards(CardClass::Strike, 2)
                .add_card(CardClass::Defend)
                .add_card(CardClass::Bash)
                .build();
            g.run_action(GainRelicAction(RelicClass::Astrolabe));
            for i in (1..=3).rev() {
                g.step_test(ChooseTransformMasterStep {
                    master_index: 0,
                    num_cards_remaining: i,
                    upgrade: true,
                });
            }
            for c in g.master_deck.iter().skip(1) {
                assert_ne!(c.borrow().class.rarity(), CardRarity::Basic);
                assert_eq!(c.borrow().upgrade_count, 1);
            }
        }
    }

    #[test]
    fn test_bottled_flame() {
        let mut g = GameBuilder::default()
            .add_card(CardClass::Strike)
            .add_card(CardClass::Bash)
            .add_card(CardClass::Defend)
            .add_card(CardClass::Impervious)
            .add_card(CardClass::Inflame)
            .add_card(CardClass::DemonForm)
            .build_with_rooms(&[RoomType::Monster]);
        g.run_action(GainRelicAction(RelicClass::BottledFlame));
        assert_eq!(
            g.valid_steps(),
            vec![
                Box::new(ChooseBottledCardStep { master_index: 0 }) as Box<dyn Step>,
                Box::new(ChooseBottledCardStep { master_index: 1 }) as Box<dyn Step>,
            ]
        );
        g.step_test(ChooseBottledCardStep { master_index: 0 });

        g.step_test(AscendStep::new(0, 0));
        assert_eq!(g.hand[0].borrow().class, CardClass::Strike);
    }

    #[test]
    fn test_bottled_lightning() {
        let mut g = GameBuilder::default()
            .add_card(CardClass::Strike)
            .add_card(CardClass::Bash)
            .add_card(CardClass::Defend)
            .add_card(CardClass::Impervious)
            .add_card(CardClass::Inflame)
            .add_card(CardClass::DemonForm)
            .build_with_rooms(&[RoomType::Monster]);
        g.run_action(GainRelicAction(RelicClass::BottledLightning));
        assert_eq!(
            g.valid_steps(),
            vec![
                Box::new(ChooseBottledCardStep { master_index: 2 }) as Box<dyn Step>,
                Box::new(ChooseBottledCardStep { master_index: 3 }) as Box<dyn Step>,
            ]
        );
        g.step_test(ChooseBottledCardStep { master_index: 2 });

        g.step_test(AscendStep::new(0, 0));
        assert_eq!(g.hand[0].borrow().class, CardClass::Defend);
    }

    #[test]
    fn test_bottled_tornado() {
        let mut g = GameBuilder::default()
            .add_card(CardClass::Strike)
            .add_card(CardClass::Bash)
            .add_card(CardClass::Defend)
            .add_card(CardClass::Impervious)
            .add_card(CardClass::Inflame)
            .add_card(CardClass::DemonForm)
            .build_with_rooms(&[RoomType::Monster]);
        g.run_action(GainRelicAction(RelicClass::BottledTornado));
        assert_eq!(
            g.valid_steps(),
            vec![
                Box::new(ChooseBottledCardStep { master_index: 4 }) as Box<dyn Step>,
                Box::new(ChooseBottledCardStep { master_index: 5 }) as Box<dyn Step>,
            ]
        );
        g.step_test(ChooseBottledCardStep { master_index: 5 });

        g.step_test(AscendStep::new(0, 0));
        assert_eq!(g.hand[0].borrow().class, CardClass::DemonForm);
    }

    #[test]
    fn test_wing_boots() {
        let mut g = GameBuilder::default()
            .add_relic(RelicClass::WingBoots)
            .build_with_rooms(&[]);
        g.map = Map::default();
        for x in 0..=1 {
            for y in 0..=5 {
                g.map.nodes[x][y].ty = Some(RoomType::Monster);
                g.map.nodes[x][y].edges = vec![x];
            }
        }
        assert_eq!(g.get_relic_value(RelicClass::WingBoots), Some(3));
        g.step_test(AscendStep::new(0, 0));
        assert_eq!(g.get_relic_value(RelicClass::WingBoots), Some(3));
        g.play_card(CardClass::DebugKillAll, None);
        g.step_test(RewardExitStep);
        assert_eq!(
            g.valid_steps(),
            vec![
                Box::new(AscendStep::new(0, 1)) as Box<dyn Step>,
                Box::new(AscendStep::wing_boots(1, 1)) as Box<dyn Step>,
            ]
        );
        g.step_test(AscendStep::wing_boots(1, 1));
        assert_eq!(g.get_relic_value(RelicClass::WingBoots), Some(2));

        g.play_card(CardClass::DebugKillAll, None);
        g.step_test(RewardExitStep);
        g.step_test(AscendStep::new(1, 2));
        assert_eq!(g.get_relic_value(RelicClass::WingBoots), Some(2));

        g.play_card(CardClass::DebugKillAll, None);
        g.step_test(RewardExitStep);
        g.step_test(AscendStep::wing_boots(0, 3));
        assert_eq!(g.get_relic_value(RelicClass::WingBoots), Some(1));

        g.play_card(CardClass::DebugKillAll, None);
        g.step_test(RewardExitStep);
        g.step_test(AscendStep::wing_boots(1, 4));
        assert_eq!(g.get_relic_value(RelicClass::WingBoots), Some(0));
        g.play_card(CardClass::DebugKillAll, None);
        g.step_test(RewardExitStep);
        assert_eq!(
            g.valid_steps(),
            vec![Box::new(AscendStep::new(1, 5)) as Box<dyn Step>,]
        );
    }

    #[test]
    fn test_frozen_eye() {
        let mut g = GameBuilder::default()
            .add_relic(RelicClass::FrozenEye)
            .add_cards(CardClass::Strike, 10)
            .add_cards(CardClass::Defend, 10)
            .add_cards(CardClass::Bash, 10)
            .build_combat();
        for _ in 0..10 {
            g.hand.clear();
            let class = g.draw_pile.get_all().last().unwrap().borrow().class;
            g.run_action(DrawAction(1));
            assert_eq!(g.hand[0].borrow().class, class);
        }
    }

    #[test]
    fn test_can_spawn() {
        let mut g = GameBuilder::default().build();
        for _ in 0..20 {
            let r = g.next_relic(RelicRarity::Uncommon);
            assert_not_matches!(
                r,
                RelicClass::BottledFlame
                    | RelicClass::BottledLightning
                    | RelicClass::BottledTornado
            );
        }
    }
}
