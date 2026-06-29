use crate::{
    action::Action,
    actions::shuffle_card_into_draw::ShuffleCardIntoDrawAction,
    cards::{CardClass, random_red_in_combat},
    game::Game,
    state::{ContinueStep, GameState, Steps},
    step::Step,
};

pub struct ChooseCardToShuffleIntoDrawAction();

impl Action for ChooseCardToShuffleIntoDrawAction {
    fn run(&self, game: &mut Game) {
        let mut classes = Vec::new();
        while classes.len() < 3 {
            let c = random_red_in_combat(&mut game.rng);
            if !classes.contains(&c) {
                classes.push(c);
            }
        }
        game.state
            .push_state(ChooseCardToShuffleIntoDrawGameState { classes });
    }
}

impl std::fmt::Debug for ChooseCardToShuffleIntoDrawAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "choose card to shuffle into draw")
    }
}

#[derive(Clone, Debug)]
struct ChooseCardToShuffleIntoDrawGameState {
    classes: Vec<CardClass>,
}

impl GameState for ChooseCardToShuffleIntoDrawGameState {
    fn clone_box(&self) -> Box<dyn GameState> {
        Box::new(self.clone())
    }
    fn valid_steps(&self, _: &Game) -> Option<Steps> {
        let mut moves = Steps::default();
        moves.push(ContinueStep);
        for &class in &self.classes {
            moves.push(ChooseCardToShuffleIntoDrawStep { class })
        }
        Some(moves)
    }
}

#[derive(Eq, PartialEq, Debug)]
pub struct ChooseCardToShuffleIntoDrawStep {
    pub class: CardClass,
}

impl Step for ChooseCardToShuffleIntoDrawStep {
    fn should_pop_state(&self) -> bool {
        true
    }

    fn run(&self, game: &mut Game) {
        game.action_queue.push_top(ShuffleCardIntoDrawAction {
            class: self.class,
            is_free: false,
        });
    }

    fn description(&self, _: &Game) -> String {
        format!("choose {:?} to shuffle into draw", self.class)
    }
}
