use crate::{
    action::Action,
    actions::{discard_card::DiscardCardAction, draw::DrawAction},
    game::Game,
    state::{GameState, Steps},
    step::Step,
};

pub struct ChooseGambleAction();

impl Action for ChooseGambleAction {
    fn run(&self, game: &mut Game) {
        if game.hand.is_empty() {
            return;
        }
        game.state.push_state(ChooseGambleGameState);
    }
}

impl std::fmt::Debug for ChooseGambleAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "choose cards to gamble")
    }
}

#[derive(Clone, Debug)]
struct ChooseGambleGameState;

impl GameState for ChooseGambleGameState {
    fn clone_box(&self) -> Box<dyn GameState> {
        Box::new(self.clone())
    }
    fn valid_steps(&self, game: &Game) -> Option<Steps> {
        let mut moves = Steps::default();
        moves.push(GambleEndStep);
        for c in 0..game.hand.len() {
            moves.push(GambleStep { hand_index: c });
        }
        Some(moves)
    }
}

#[derive(Eq, PartialEq, Debug)]
pub struct GambleStep {
    pub hand_index: usize,
}

impl Step for GambleStep {
    fn should_pop_state(&self) -> bool {
        false
    }

    fn run(&self, game: &mut Game) {
        game.chosen_cards.push(game.hand.remove(self.hand_index));
    }

    fn description(&self, game: &Game) -> String {
        format!(
            "gamble {} ({:?})",
            self.hand_index,
            game.hand[self.hand_index].borrow()
        )
    }
}

#[derive(Eq, PartialEq, Debug)]
pub struct GambleEndStep;

impl Step for GambleEndStep {
    fn should_pop_state(&self) -> bool {
        true
    }

    fn run(&self, game: &mut Game) {
        let count = game.chosen_cards.len() as i32;
        game.action_queue.push_top(DrawAction(count));
        while let Some(c) = game.chosen_cards.pop() {
            game.action_queue.push_top(DiscardCardAction(c));
        }
    }

    fn description(&self, _: &Game) -> String {
        "end gamble cards".to_owned()
    }
}
