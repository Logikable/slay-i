use crate::{
    action::Action,
    actions::forethought::ForethoughtAction,
    game::Game,
    state::{GameState, Steps},
    step::Step,
};

pub struct ChooseForethoughtAnyAction();

impl Action for ChooseForethoughtAnyAction {
    fn run(&self, g: &mut Game) {
        if g.hand.is_empty() {
            return;
        }
        g.state.push_state(ForethoughtAnyGameState);
    }
}

impl std::fmt::Debug for ChooseForethoughtAnyAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "choose forethought any")
    }
}

#[derive(Clone, Debug)]
struct ForethoughtAnyGameState;

impl GameState for ForethoughtAnyGameState {
    fn clone_box(&self) -> Box<dyn GameState> {
        Box::new(self.clone())
    }
    fn valid_steps(&self, game: &Game) -> Option<Steps> {
        let mut moves = Steps::default();
        moves.push(ForethoughtAnyEndStep);
        for c in 0..game.hand.len() {
            moves.push(ForethoughtAnyStep { hand_index: c });
        }
        Some(moves)
    }
}

#[derive(Eq, PartialEq, Debug)]
pub struct ForethoughtAnyStep {
    pub hand_index: usize,
}

impl Step for ForethoughtAnyStep {
    fn should_pop_state(&self) -> bool {
        true
    }

    fn run(&self, game: &mut Game) {
        game.chosen_cards.push(game.hand.remove(self.hand_index));
        game.state.push_state(ForethoughtAnyGameState);
    }

    fn description(&self, game: &Game) -> String {
        format!(
            "forethought {} ({:?})",
            self.hand_index,
            game.hand[self.hand_index].borrow()
        )
    }
}

#[derive(Eq, PartialEq, Debug)]
pub struct ForethoughtAnyEndStep;

impl Step for ForethoughtAnyEndStep {
    fn should_pop_state(&self) -> bool {
        true
    }

    fn run(&self, game: &mut Game) {
        while !game.chosen_cards.is_empty() {
            game.action_queue
                .push_top(ForethoughtAction(game.chosen_cards.remove(0)));
        }
    }

    fn description(&self, _: &Game) -> String {
        "end forethought cards".to_owned()
    }
}
