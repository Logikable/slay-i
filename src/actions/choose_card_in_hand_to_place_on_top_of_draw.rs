use crate::{
    action::Action,
    actions::place_card_on_top_of_draw::PlaceCardOnTopOfDrawAction,
    game::Game,
    state::{GameState, Steps},
    step::Step,
};

pub struct ChooseCardInHandToPlaceOnTopOfDrawAction();

impl Action for ChooseCardInHandToPlaceOnTopOfDrawAction {
    fn run(&self, game: &mut Game) {
        match game.hand.len() {
            0 => {}
            1 => game
                .action_queue
                .push_top(PlaceCardOnTopOfDrawAction(game.hand.pop().unwrap())),
            _ => game
                .state
                .push_state(ChooseCardInHandToPlaceOnTopOfDrawGameState),
        }
    }
}

impl std::fmt::Debug for ChooseCardInHandToPlaceOnTopOfDrawAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "move one card in hand on top of draw")
    }
}

#[derive(Clone, Debug)]
struct ChooseCardInHandToPlaceOnTopOfDrawGameState;

impl GameState for ChooseCardInHandToPlaceOnTopOfDrawGameState {
    fn clone_box(&self) -> Box<dyn GameState> {
        Box::new(self.clone())
    }
    fn valid_steps(&self, game: &Game) -> Option<Steps> {
        let mut moves = Steps::default();
        for i in 0..game.hand.len() {
            moves.push(PlaceCardInHandOnTopOfDrawStep { hand_index: i });
        }
        Some(moves)
    }
}

#[derive(Eq, PartialEq, Debug)]
pub struct PlaceCardInHandOnTopOfDrawStep {
    pub hand_index: usize,
}

impl Step for PlaceCardInHandOnTopOfDrawStep {
    fn should_pop_state(&self) -> bool {
        true
    }

    fn run(&self, game: &mut Game) {
        game.action_queue.push_top(PlaceCardOnTopOfDrawAction(
            game.hand.remove(self.hand_index),
        ));
    }

    fn description(&self, game: &Game) -> String {
        format!(
            "place card on top of draw {} ({:?})",
            self.hand_index,
            game.hand[self.hand_index].borrow()
        )
    }
}
