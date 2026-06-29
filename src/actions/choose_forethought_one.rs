use crate::{
    action::Action,
    actions::forethought::ForethoughtAction,
    game::Game,
    state::{GameState, Steps},
    step::Step,
};

pub struct ChooseForethoughtOneAction();

impl Action for ChooseForethoughtOneAction {
    fn run(&self, game: &mut Game) {
        match game.hand.len() {
            0 => {}
            1 => game
                .action_queue
                .push_top(ForethoughtAction(game.hand.pop().unwrap())),
            _ => game.state.push_state(ForethoughtOneGameState),
        }
    }
}

impl std::fmt::Debug for ChooseForethoughtOneAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "choose forethought one")
    }
}

#[derive(Clone, Debug)]
struct ForethoughtOneGameState;

impl GameState for ForethoughtOneGameState {
    fn clone_box(&self) -> Box<dyn GameState> {
        Box::new(self.clone())
    }
    fn valid_steps(&self, game: &Game) -> Option<Steps> {
        let mut moves = Steps::default();
        for c in 0..game.hand.len() {
            moves.push(ForethoughtOneStep { hand_index: c });
        }
        Some(moves)
    }
}

#[derive(Eq, PartialEq, Debug)]
pub struct ForethoughtOneStep {
    pub hand_index: usize,
}

impl Step for ForethoughtOneStep {
    fn should_pop_state(&self) -> bool {
        true
    }

    fn run(&self, game: &mut Game) {
        let c = game.hand.remove(self.hand_index);
        game.action_queue.push_top(ForethoughtAction(c));
    }

    fn description(&self, game: &Game) -> String {
        format!(
            "forethought {} ({:?})",
            self.hand_index,
            game.hand[self.hand_index].borrow()
        )
    }
}
