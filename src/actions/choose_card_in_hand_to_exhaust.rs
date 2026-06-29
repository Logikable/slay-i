use crate::{
    action::Action,
    actions::exhaust_card::ExhaustCardAction,
    game::Game,
    state::{GameState, Steps},
    step::Step,
};

pub struct ChooseCardInHandToExhaustAction();

impl Action for ChooseCardInHandToExhaustAction {
    fn run(&self, game: &mut Game) {
        match game.hand.len() {
            0 => {}
            1 => game
                .action_queue
                .push_top(ExhaustCardAction(game.hand.pop().unwrap())),
            _ => game.state.push_state(ChooseExhaustOneCardInHandGameState),
        }
    }
}

impl std::fmt::Debug for ChooseCardInHandToExhaustAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "choose card in hand to exhaust")
    }
}

#[derive(Clone, Debug)]
struct ChooseExhaustOneCardInHandGameState;

impl GameState for ChooseExhaustOneCardInHandGameState {
    fn clone_box(&self) -> Box<dyn GameState> {
        Box::new(self.clone())
    }
    fn valid_steps(&self, game: &Game) -> Option<Steps> {
        let mut moves = Steps::default();
        for i in 0..game.hand.len() {
            moves.push(ExhaustOneCardInHandStep { hand_index: i });
        }
        Some(moves)
    }
}

#[derive(Eq, PartialEq, Debug)]
pub struct ExhaustOneCardInHandStep {
    pub hand_index: usize,
}

impl Step for ExhaustOneCardInHandStep {
    fn should_pop_state(&self) -> bool {
        true
    }

    fn run(&self, game: &mut Game) {
        game.action_queue
            .push_top(ExhaustCardAction(game.hand.remove(self.hand_index)));
    }

    fn description(&self, game: &Game) -> String {
        format!(
            "exhaust {} ({:?})",
            self.hand_index,
            game.hand[self.hand_index].borrow()
        )
    }
}
