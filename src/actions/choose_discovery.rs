use crate::{
    action::Action,
    actions::discovery::DiscoveryAction,
    cards::{
        CardClass, random_colorless_in_combat, random_red_attack_in_combat, random_red_in_combat,
        random_red_power_in_combat, random_red_skill_in_combat,
    },
    game::Game,
    state::{GameState, Steps},
    step::Step,
};

pub enum ChooseDiscoveryType {
    Red,
    RedAttack,
    RedSkill,
    RedPower,
    Colorless,
}

pub struct ChooseDiscoveryAction {
    pub ty: ChooseDiscoveryType,
    pub amount: i32,
    pub is_free: bool,
}

impl Action for ChooseDiscoveryAction {
    fn run(&self, game: &mut Game) {
        let mut classes = Vec::new();
        while classes.len() < 3 {
            let c = match self.ty {
                ChooseDiscoveryType::Red => random_red_in_combat(&mut game.rng),
                ChooseDiscoveryType::RedAttack => random_red_attack_in_combat(&mut game.rng),
                ChooseDiscoveryType::RedSkill => random_red_skill_in_combat(&mut game.rng),
                ChooseDiscoveryType::RedPower => random_red_power_in_combat(&mut game.rng),
                ChooseDiscoveryType::Colorless => random_colorless_in_combat(&mut game.rng),
            };
            if !classes.contains(&c) {
                classes.push(c);
            }
        }
        game.state.push_state(ChooseDiscoveryGameState {
            classes,
            amount: self.amount,
            is_free: self.is_free,
        });
    }
}

impl std::fmt::Debug for ChooseDiscoveryAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "choose discovery")
    }
}

#[derive(Clone, Debug)]
struct ChooseDiscoveryGameState {
    classes: Vec<CardClass>,
    amount: i32,
    is_free: bool,
}

impl GameState for ChooseDiscoveryGameState {
    fn clone_box(&self) -> Box<dyn GameState> {
        Box::new(self.clone())
    }
    fn valid_steps(&self, _: &Game) -> Option<Steps> {
        let mut moves = Steps::default();
        for &class in &self.classes {
            moves.push(DiscoveryStep {
                class,
                amount: self.amount,
                is_free: self.is_free,
            });
        }
        Some(moves)
    }
}

#[derive(Eq, PartialEq, Debug)]
struct DiscoveryStep {
    class: CardClass,
    amount: i32,
    is_free: bool,
}

impl Step for DiscoveryStep {
    fn should_pop_state(&self) -> bool {
        true
    }

    fn run(&self, game: &mut Game) {
        game.action_queue.push_top(DiscoveryAction {
            class: self.class,
            amount: self.amount,
            is_free: self.is_free,
        });
    }

    fn description(&self, _: &Game) -> String {
        format!("discovery {:?}", self.class)
    }
}
