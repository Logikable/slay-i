mod attacks;
mod curses;
mod powers;
mod skills;
mod statuses;

use lazy_static::lazy_static;

use crate::{
    card::CardPlayInfo,
    game::{Game, Rand},
    rng::rand_slice,
};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CardType {
    Attack,
    Skill,
    Power,
    Status,
    Curse,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CardRarity {
    Basic,
    Common,
    Uncommon,
    Rare,
    Special,
    Curse,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CardColor {
    Red,
    Colorless,
    Curse,
    Special,
}

macro_rules! c {
    ($($name:ident => ($rarity:expr, $ty:expr, $color:expr, $cost:expr, $behavior:expr, $exhausts:expr)),+,) => {
        #[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
        pub enum CardClass {
            $(
                $name,
            )+
        }
        impl CardClass {
            pub fn rarity(&self) -> CardRarity {
                use CardRarity::*;
                match self {
                    $(Self::$name => $rarity,)+
                }
            }
            pub fn ty(&self) -> CardType {
                use CardType::*;
                match self {
                    $(Self::$name => $ty,)+
                }
            }
            pub fn color(&self) -> CardColor {
                use CardColor::*;
                match self {
                    $(Self::$name => $color,)+
                }
            }
            pub fn base_cost(&self) -> CardCost {
                use CardCost::*;
                fn cost(c: i32) -> CardCost {
                    CardCost::Cost {base_cost:c, temporary_cost: None, free_to_play_once: false }
                }
                match self {
                    $(Self::$name => $cost,)+
                }
            }
            pub fn behavior(&self) -> CardBehavior {
                match self {
                    $(Self::$name => $behavior,)+
                }
            }
            pub fn base_exhausts(&self) -> bool {
                match self {
                    $(Self::$name => $exhausts,)+
                }
            }
        }
        impl CardClass {
            pub fn all() -> Vec<Self> {
                vec![$(Self::$name,)+]
            }
        }
    };
}

fn noop_behavior(_: &mut Game, _: &CardPlayInfo) {}

c!(
    // Basic
    Strike => (Basic, Attack, Red, cost(1), attacks::strike_behavior, false),
    Defend => (Basic, Skill, Red, cost(1), skills::defend_behavior, false),
    Bash => (Basic, Attack, Red, cost(2), attacks::bash_behavior, false),
    // Common attacks
    PommelStrike => (Common, Attack, Red, cost(1), attacks::pommel_strike_behavior, false),
    TwinStrike => (Common, Attack, Red, cost(1), attacks::twin_strike_behavior, false),
    Clothesline => (Common, Attack, Red, cost(2), attacks::clothesline_behavior, false),
    Cleave => (Common, Attack, Red, cost(1), attacks::cleave_behavior, false),
    Thunderclap => (Common, Attack, Red, cost(1), attacks::thunderclap_behavior, false),
    BodySlam => (Common, Attack, Red, cost(1), attacks::body_slam_behavior, false),
    IronWave => (Common, Attack, Red, cost(1), attacks::iron_wave_behavior, false),
    WildStrike => (Common, Attack, Red, cost(1), attacks::wild_strike_behavior, false),
    Headbutt => (Common, Attack, Red, cost(1), attacks::headbutt_behavior, false),
    SwordBoomerang => (Common, Attack, Red, cost(1), attacks::sword_boomerang_behavior, false),
    PerfectedStrike => (Common, Attack, Red, cost(2), attacks::perfected_strike_behavior, false),
    HeavyBlade => (Common, Attack, Red, cost(2), attacks::heavy_blade_behavior, false),
    Anger => (Common, Attack, Red, cost(0), attacks::anger_behavior, false),
    Clash => (Common, Attack, Red, cost(0), attacks::clash_behavior, false),
    // Common skills
    Armaments => (Common, Skill, Red, cost(1), skills::armaments_behavior, false),
    Flex => (Common, Skill, Red, cost(0), skills::flex_behavior, false),
    TrueGrit => (Common, Skill, Red, cost(1), skills::true_grit_behavior, false),
    ShrugItOff => (Common, Skill, Red, cost(1), skills::shrug_it_off_behavior, false),
    Havoc => (Common, Skill, Red, cost(1), skills::havoc_behavior, false),
    Warcry => (Common, Skill, Red, cost(0), skills::warcry_behavior, true),
    // Uncommon attacks
    SearingBlow => (Uncommon, Attack, Red, cost(2), attacks::searing_blow_behavior, false),
    Whirlwind => (Uncommon, Attack, Red, X, attacks::whirlwind_behavior, false),
    Rampage => (Uncommon, Attack, Red, cost(1), attacks::rampage_behavior, false),
    Uppercut => (Uncommon, Attack, Red, cost(2), attacks::uppercut_behavior, false),
    SeverSoul => (Uncommon, Attack, Red, cost(2), attacks::sever_soul_behavior, false),
    Carnage => (Uncommon, Attack, Red, cost(2), attacks::carnage_behavior, false),
    Hemokinesis => (Uncommon, Attack, Red, cost(1), attacks::hemokinesis_behavior, false),
    Dropkick => (Uncommon, Attack, Red, cost(1), attacks::dropkick_behavior, false),
    Pummel => (Uncommon, Attack, Red, cost(1), attacks::pummel_behavior, true),
    BloodForBlood => (Uncommon, Attack, Red, cost(4), attacks::blood_for_blood_behavior, false),
    RecklessCharge => (Uncommon, Attack, Red, cost(0), attacks::reckless_charge_behavior, false),
    // Uncommon skills
    GhostlyArmor => (Uncommon, Skill, Red, cost(1), skills::ghostly_armor_behavior, false),
    Bloodletting => (Uncommon, Skill, Red, cost(0), skills::bloodletting_behavior, false),
    Sentinel => (Uncommon, Skill, Red, cost(1), skills::sentinel_behavior, false),
    SpotWeakness => (Uncommon, Skill, Red, cost(1), skills::spot_weakness_behavior, false),
    DualWield => (Uncommon, Skill, Red, cost(1), skills::dual_wield_behavior, false),
    BattleTrance => (Uncommon, Skill, Red, cost(0), skills::battle_trance_behavior, false),
    Disarm => (Uncommon, Skill, Red, cost(1), skills::disarm_behavior, true),
    Rage => (Uncommon, Skill, Red, cost(0), skills::rage_behavior, false),
    Intimidate => (Uncommon, Skill, Red, cost(0), skills::intimidate_behavior, true),
    FlameBarrier => (Uncommon, Skill, Red, cost(2), skills::flame_barrier_behavior, false),
    Shockwave => (Uncommon, Skill, Red, cost(2), skills::shockwave_behavior, true),
    Entrench => (Uncommon, Skill, Red, cost(2), skills::entrench_behavior, false),
    BurningPact => (Uncommon, Skill, Red, cost(1), skills::burning_pact_behavior, false),
    SeeingRed => (Uncommon, Skill, Red, cost(1), skills::seeing_red_behavior, true),
    PowerThrough => (Uncommon, Skill, Red, cost(1), skills::power_through_behavior, false),
    InfernalBlade => (Uncommon, Skill, Red, cost(1), skills::infernal_blade_behavior, true),
    SecondWind => (Uncommon, Skill, Red, cost(1), skills::second_wind_behavior, false),
    // Uncommon powers
    Inflame => (Uncommon, Power, Red, cost(1), powers::inflame_behavior, false),
    FeelNoPain => (Uncommon, Power, Red, cost(1), powers::feel_no_pain_behavior, false),
    DarkEmbrace => (Uncommon, Power, Red, cost(2), powers::dark_embrace_behavior, false),
    Evolve => (Uncommon, Power, Red, cost(1), powers::evolve_behavior, false),
    Metallicize => (Uncommon, Power, Red, cost(1), powers::metallicize_behavior, false),
    Combust => (Uncommon, Power, Red, cost(1), powers::combust_behavior, false),
    FireBreathing => (Uncommon, Power, Red, cost(1), powers::firebreathing_behavior, false),
    Rupture => (Uncommon, Power, Red, cost(1), powers::rupture_behavior, false),
    // Rare attacks
    Reaper => (Rare, Attack, Red, cost(2), attacks::reaper_behavior, true),
    Immolate => (Rare, Attack, Red, cost(2), attacks::immolate_behavior, false),
    Bludgeon => (Rare, Attack, Red, cost(3), attacks::bludgeon_behavior, false),
    Feed => (Rare, Attack, Red, cost(1), attacks::feed_behavior, true),
    FiendFire => (Rare, Attack, Red, cost(2), attacks::fiend_fire_behavior, true),
    // Rare skills
    LimitBreak => (Rare, Skill, Red, cost(1), skills::limit_break_behavior, true),
    Impervious => (Rare, Skill, Red, cost(2), skills::impervious_behavior, true),
    DoubleTap => (Rare, Skill, Red, cost(1), skills::double_tap_behavior, false),
    Offering => (Rare, Skill, Red, cost(0), skills::offering_behavior, true),
    Exhume => (Rare, Skill, Red, cost(1), skills::exhume_behavior, true),
    // Rare powers
    Brutality => (Rare, Power, Red, cost(0), powers::brutality_behavior, false),
    DemonForm => (Rare, Power, Red, cost(3), powers::demon_form_behavior, false),
    Barricade => (Rare, Power, Red, cost(3), powers::barricade_behavior, false),
    Corruption => (Rare, Power, Red, cost(3), powers::corruption_behavior, false),
    Juggernaut => (Rare, Power, Red, cost(2), powers::juggernaut_behavior, false),
    Berserk => (Rare, Power, Red, cost(0), powers::berserk_behavior, false),
    // Colorless uncommon attacks
    SwiftStrike => (Uncommon, Attack, Colorless, cost(0), attacks::swift_strike_behavior, false),
    FlashOfSteel => (Uncommon, Attack, Colorless, cost(0), attacks::flash_of_steel_behavior, false),
    DramaticEntrance => (Uncommon, Attack, Colorless, cost(0), attacks::dramatic_entrance_behavior, true),
    MindBlast => (Uncommon, Attack, Colorless, cost(2), attacks::mind_blast_behavior, false),
    // Colorless uncommon skills
    GoodInstincts => (Uncommon, Skill, Colorless, cost(0), skills::good_instincts_behavior, false),
    Finesse => (Uncommon, Skill, Colorless, cost(0), skills::finesse_behavior, false),
    Enlightenment => (Uncommon, Skill, Colorless, cost(0), skills::enlightenment_behavior, false),
    Impatience => (Uncommon, Skill, Colorless, cost(0), skills::impatience_behavior, false),
    JackOfAllTrades => (Uncommon, Skill, Colorless, cost(0), skills::jack_of_all_trades_behavior, true),
    Forethought => (Uncommon, Skill, Colorless, cost(0), skills::forethought_behavior, false),
    BandageUp => (Uncommon, Skill, Colorless, cost(0), skills::bandage_up_behavior, true),
    Blind => (Uncommon, Skill, Colorless, cost(0), skills::blind_behavior, false),
    Trip => (Uncommon, Skill, Colorless, cost(0), skills::trip_behavior, false),
    Discovery => (Uncommon, Skill, Colorless, cost(1), skills::discovery_behavior, true),
    DeepBreath => (Uncommon, Skill, Colorless, cost(0), skills::deep_breath_behavior, false),
    DarkShackles => (Uncommon, Skill, Colorless, cost(0), skills::dark_shackles_behavior, true),
    PanicButton => (Uncommon, Skill, Colorless, cost(0), skills::panic_button_behavior, true),
    Purity => (Uncommon, Skill, Colorless, cost(0), skills::purity_behavior, true),
    Panacea => (Uncommon, Skill, Colorless, cost(0), skills::panacea_behavior, true),
    Madness => (Uncommon, Skill, Colorless, cost(1), skills::madness_behavior, true),
    // Colorless rare attacks
    HandOfGreed => (Rare, Attack, Colorless, cost(2), attacks::hand_of_greed_behavior, false),
    // Colorless rare skills
    Bomb => (Rare, Skill, Colorless, cost(2), skills::bomb_behavior, false),
    Apotheosis => (Rare, Skill, Colorless, cost(2), skills::apotheosis_behavior, true),
    ThinkingAhead => (Rare, Skill, Colorless, cost(0), skills::thinking_ahead_behavior, true),
    SecretTechnique => (Rare, Skill, Colorless, cost(0), skills::secret_technique_behavior, true),
    SecretWeapon => (Rare, Skill, Colorless, cost(0), skills::secret_weapon_behavior, true),
    Metamorphosis => (Rare, Skill, Colorless, cost(2), skills::metamorphosis_behavior, true),
    Chrysalis => (Rare, Skill, Colorless, cost(2), skills::chrysalis_behavior, true),
    Transmutation => (Rare, Skill, Colorless, X, skills::transmutation_behavior, true),
    MasterOfStrategy => (Rare, Skill, Colorless, cost(0), skills::master_of_strategy_behavior, true),
    Violence => (Rare, Skill, Colorless, cost(0), skills::violence_behavior, true),
    // Colorless rare powers
    Panache => (Rare, Power, Colorless, cost(0), powers::panache_behavior, false),
    SadisticNature => (Rare, Power, Colorless, cost(0), powers::sadistic_nature_behavior, false),
    Mayhem => (Rare, Power, Colorless, cost(2), powers::mayhem_behavior, false),
    Magnetism => (Rare, Power, Colorless, cost(2), powers::magnetism_behavior, false),
    // Colorless special attacks
    Bite => (Special, Attack, Special, cost(1), attacks::bite_behavior, false),
    RitualDagger => (Special, Attack, Special, cost(1), attacks::ritual_dagger_behavior, true),
    // Colorless special skills
    Apparition => (Special, Skill, Special, cost(1), skills::apparition_behavior, true),
    Jax => (Special, Skill, Special, cost(0), skills::jax_behavior, false),
    // Statuses
    Wound => (Special, Status, Special, Zero, noop_behavior, true),
    Dazed => (Special, Status, Special, Zero, noop_behavior, true),
    Slimed => (Special, Status, Special, cost(1), noop_behavior, true),
    Burn => (Special, Status, Special, Zero, noop_behavior, true),
    BurnPlus => (Special, Status, Special, Zero, noop_behavior, true),
    Void => (Special, Status, Special, Zero, noop_behavior, true),
    // Curses
    AscendersBane => (Curse, Curse, Curse, Zero, noop_behavior, true),
    CurseOfTheBell => (Curse, Curse, Curse, Zero, noop_behavior, true),
    Clumsy => (Curse, Curse, Curse, Zero, noop_behavior, true),
    Injury => (Curse, Curse, Curse, Zero, noop_behavior, true),
    Writhe => (Curse, Curse, Curse, Zero, noop_behavior, true),
    Parasite => (Curse, Curse, Curse, Zero, noop_behavior, true),
    Shame => (Curse, Curse, Curse, Zero, noop_behavior, true),
    Doubt => (Curse, Curse, Curse, Zero, noop_behavior, true),
    Decay => (Curse, Curse, Curse, Zero, noop_behavior, true),
    Regret => (Curse, Curse, Curse, Zero, noop_behavior, true),
    Pain => (Curse, Curse, Curse, Zero, noop_behavior, true),
    Normality => (Curse, Curse, Curse, Zero, noop_behavior, true),
    Necronomicurse => (Curse, Curse, Curse, Zero, noop_behavior, true),
    // Other
    DebugKill => (Special, Attack, Special, cost(0), attacks::debug_kill_behavior, false),
    DebugKillAll => (Special, Attack, Special, cost(0), attacks::debug_kill_all_behavior, false),
    TestAttack => (Special, Attack, Special, cost(0), noop_behavior, false),
    TestSkill => (Special, Skill, Special, cost(0), noop_behavior, false),
    TestPower => (Special, Power, Special, cost(0), noop_behavior, false),
);

pub type CardBehavior = fn(&mut Game, &CardPlayInfo);
pub type CardEndOfTurnBehavior = fn(&mut Game);

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CardCost {
    Cost {
        base_cost: i32,
        temporary_cost: Option<i32>,
        free_to_play_once: bool,
    },
    X,
    Zero,
}

impl CardClass {
    pub fn can_upgrade_forever(&self) -> bool {
        matches!(self, CardClass::SearingBlow)
    }
    pub fn end_of_turn_in_hand_behavior(&self) -> Option<CardEndOfTurnBehavior> {
        use CardClass::*;
        match self {
            Burn => Some(statuses::burn_behavior),
            BurnPlus => Some(statuses::burn_plus_behavior),
            Regret => Some(curses::regret_behavior),
            Decay => Some(curses::decay_behavior),
            Shame => Some(curses::shame_behavior),
            Doubt => Some(curses::doubt_behavior),
            _ => None,
        }
    }
    pub fn upgrade_removes_exhaust(&self) -> bool {
        use CardClass::*;
        matches!(
            self,
            LimitBreak | Discovery | ThinkingAhead | SecretTechnique | SecretWeapon
        )
    }
    pub fn upgrade_cost(&self, cur_cost: i32) -> Option<i32> {
        use CardClass::*;
        match self {
            BodySlam | Havoc | InfernalBlade | Exhume | SeeingRed | Madness => Some(0),
            DarkEmbrace | Entrench | MindBlast | Mayhem | Magnetism | Apotheosis => Some(1),
            Corruption | Barricade => Some(2),
            BloodForBlood => Some((cur_cost - 1).max(0)),
            _ => None,
        }
    }
    pub fn can_be_generated_in_combat(&self) -> bool {
        use CardClass::*;
        !matches!(self, Feed | Reaper | Bite | BandageUp)
    }
    pub fn is_strike(&self) -> bool {
        use CardClass::*;
        matches!(
            self,
            Strike | SwiftStrike | PommelStrike | PerfectedStrike | TwinStrike | WildStrike
        )
    }
}

lazy_static! {
    static ref ALL_COLORLESS: Vec<CardClass> = CardClass::all()
        .into_iter()
        .filter(|c| c.color() == CardColor::Colorless)
        .collect();
    static ref ALL_UNCOMMON_COLORLESS: Vec<CardClass> = ALL_COLORLESS
        .iter()
        .copied()
        .filter(|c| c.rarity() == CardRarity::Uncommon)
        .collect();
    static ref ALL_RARE_COLORLESS: Vec<CardClass> = ALL_COLORLESS
        .iter()
        .copied()
        .filter(|c| c.rarity() == CardRarity::Rare)
        .collect();
    static ref ALL_COLORLESS_IN_COMBAT: Vec<CardClass> = ALL_COLORLESS
        .iter()
        .copied()
        .filter(|c| c.can_be_generated_in_combat())
        .collect();
    static ref ALL_NON_BASIC_RED: Vec<CardClass> = CardClass::all()
        .iter()
        .copied()
        .filter(|c| c.color() == CardColor::Red)
        .filter(|c| c.rarity() != CardRarity::Basic)
        .collect();
    static ref ALL_COMMON_RED: Vec<CardClass> = ALL_NON_BASIC_RED
        .iter()
        .copied()
        .filter(|c| c.rarity() == CardRarity::Common)
        .collect();
    static ref ALL_UNCOMMON_RED: Vec<CardClass> = ALL_NON_BASIC_RED
        .iter()
        .copied()
        .filter(|c| c.rarity() == CardRarity::Uncommon)
        .collect();
    static ref ALL_RARE_RED: Vec<CardClass> = ALL_NON_BASIC_RED
        .iter()
        .copied()
        .filter(|c| c.rarity() == CardRarity::Rare)
        .collect();
    static ref ALL_NON_BASIC_RED_ATTACKS: Vec<CardClass> = ALL_NON_BASIC_RED
        .iter()
        .copied()
        .filter(|c| c.ty() == CardType::Attack)
        .collect();
    static ref ALL_NON_BASIC_RED_SKILLS: Vec<CardClass> = ALL_NON_BASIC_RED
        .iter()
        .copied()
        .filter(|c| c.ty() == CardType::Skill)
        .collect();
    static ref ALL_NON_BASIC_RED_POWERS: Vec<CardClass> = ALL_NON_BASIC_RED
        .iter()
        .copied()
        .filter(|c| c.ty() == CardType::Power)
        .collect();
    static ref ALL_RED_IN_COMBAT: Vec<CardClass> = ALL_NON_BASIC_RED
        .iter()
        .copied()
        .filter(|c| c.can_be_generated_in_combat())
        .collect();
    static ref ALL_RED_ATTACKS_IN_COMBAT: Vec<CardClass> = ALL_RED_IN_COMBAT
        .iter()
        .copied()
        .filter(|c| c.ty() == CardType::Attack)
        .collect();
    static ref ALL_RED_SKILLS_IN_COMBAT: Vec<CardClass> = ALL_RED_IN_COMBAT
        .iter()
        .copied()
        .filter(|c| c.ty() == CardType::Skill)
        .collect();
    static ref ALL_RED_POWERS_IN_COMBAT: Vec<CardClass> = ALL_RED_IN_COMBAT
        .iter()
        .copied()
        .filter(|c| c.ty() == CardType::Power)
        .collect();
    static ref ALL_CURSES: Vec<CardClass> = CardClass::all()
        .iter()
        .copied()
        .filter(|c| c.ty() == CardType::Curse)
        .filter(|&c| c != CardClass::AscendersBane
            && c != CardClass::CurseOfTheBell
            && c != CardClass::Necronomicurse)
        .collect();
}

pub fn random_red(rng: &mut Rand) -> CardClass {
    rand_slice(rng, &ALL_NON_BASIC_RED)
}

pub fn random_common_red(rng: &mut Rand) -> CardClass {
    rand_slice(rng, &ALL_COMMON_RED)
}

pub fn random_uncommon_red(rng: &mut Rand) -> CardClass {
    rand_slice(rng, &ALL_UNCOMMON_RED)
}

pub fn random_rare_red(rng: &mut Rand) -> CardClass {
    rand_slice(rng, &ALL_RARE_RED)
}

pub fn random_red_attack(rng: &mut Rand) -> CardClass {
    rand_slice(rng, &ALL_NON_BASIC_RED_ATTACKS)
}

pub fn random_red_skill(rng: &mut Rand) -> CardClass {
    rand_slice(rng, &ALL_NON_BASIC_RED_SKILLS)
}

pub fn random_red_power(rng: &mut Rand) -> CardClass {
    rand_slice(rng, &ALL_NON_BASIC_RED_POWERS)
}

pub fn random_red_in_combat(rng: &mut Rand) -> CardClass {
    rand_slice(rng, &ALL_RED_IN_COMBAT)
}

pub fn random_red_attack_in_combat(rng: &mut Rand) -> CardClass {
    rand_slice(rng, &ALL_RED_ATTACKS_IN_COMBAT)
}

pub fn random_red_skill_in_combat(rng: &mut Rand) -> CardClass {
    rand_slice(rng, &ALL_RED_SKILLS_IN_COMBAT)
}

pub fn random_red_power_in_combat(rng: &mut Rand) -> CardClass {
    rand_slice(rng, &ALL_RED_POWERS_IN_COMBAT)
}

pub fn random_colorless(rng: &mut Rand) -> CardClass {
    rand_slice(rng, &ALL_COLORLESS)
}

pub fn random_colorless_in_combat(rng: &mut Rand) -> CardClass {
    rand_slice(rng, &ALL_COLORLESS_IN_COMBAT)
}

pub fn random_uncommon_colorless(rng: &mut Rand) -> CardClass {
    rand_slice(rng, &ALL_UNCOMMON_COLORLESS)
}

pub fn random_rare_colorless(rng: &mut Rand) -> CardClass {
    rand_slice(rng, &ALL_RARE_COLORLESS)
}

pub fn random_curse(rng: &mut Rand) -> CardClass {
    rand_slice(rng, &ALL_CURSES)
}

pub fn transformed(class: CardClass, rng: &mut Rand) -> CardClass {
    loop {
        let new = match class.color() {
            CardColor::Red => random_red(rng),
            CardColor::Colorless => random_colorless(rng),
            CardColor::Curse => random_curse(rng),
            CardColor::Special => unreachable!(),
        };
        if new != class {
            return new;
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        cards::{CardClass, CardColor, CardRarity, transformed},
        game::{GameBuilder, Rand},
    };

    #[test]
    fn test_transformed() {
        let mut rng = Rand::default();
        for _ in 0..100 {
            {
                let t = transformed(CardClass::Strike, &mut rng);
                assert_eq!(t.color(), CardColor::Red);
                assert!(
                    t.rarity() == CardRarity::Common
                        || t.rarity() == CardRarity::Uncommon
                        || t.rarity() == CardRarity::Rare
                );
            }
            {
                let t = transformed(CardClass::Impervious, &mut rng);
                assert_eq!(t.color(), CardColor::Red);
                assert!(
                    t.rarity() == CardRarity::Common
                        || t.rarity() == CardRarity::Uncommon
                        || t.rarity() == CardRarity::Rare
                );
                assert_ne!(t, CardClass::Impervious);
            }
            {
                let t = transformed(CardClass::FlashOfSteel, &mut rng);
                assert_eq!(t.color(), CardColor::Colorless);
                assert!(t.rarity() == CardRarity::Uncommon || t.rarity() == CardRarity::Rare);
                assert_ne!(t, CardClass::FlashOfSteel);
            }
            {
                let t = transformed(CardClass::AscendersBane, &mut rng);
                assert_eq!(t.color(), CardColor::Curse);
                assert_eq!(t.rarity(), CardRarity::Curse);
            }
            {
                let t = transformed(CardClass::Injury, &mut rng);
                assert_eq!(t.color(), CardColor::Curse);
                assert_eq!(t.rarity(), CardRarity::Curse);
                assert_ne!(t, CardClass::AscendersBane);
                assert_ne!(t, CardClass::Injury);
            }
        }
    }

    #[test]
    fn test_innate() {
        let g = GameBuilder::default()
            .add_card(CardClass::MindBlast)
            .add_card_upgraded(CardClass::Brutality)
            .add_card(CardClass::DramaticEntrance)
            .add_card(CardClass::Writhe)
            .add_cards(CardClass::Strike, 50)
            .build_combat();
        assert!(
            g.hand
                .iter()
                .any(|c| c.borrow().class == CardClass::MindBlast)
        );
        assert!(
            g.hand
                .iter()
                .any(|c| c.borrow().class == CardClass::Brutality)
        );
        assert!(
            g.hand
                .iter()
                .any(|c| c.borrow().class == CardClass::DramaticEntrance)
        );
        assert!(g.hand.iter().any(|c| c.borrow().class == CardClass::Writhe));
    }

    #[test]
    fn test_innate2() {
        {
            let g = GameBuilder::default()
                .add_cards(CardClass::DramaticEntrance, 7)
                .add_cards(CardClass::Strike, 50)
                .build_combat();
            assert_eq!(g.hand.len(), 7);
        }
        {
            let g = GameBuilder::default()
                .add_cards(CardClass::DramaticEntrance, 11)
                .add_cards(CardClass::Strike, 50)
                .build_combat();
            assert_eq!(g.hand.len(), 10);
        }
    }
}
