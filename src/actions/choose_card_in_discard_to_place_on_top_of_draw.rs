use crate::{
    action::Action,
    actions::place_card_on_top_of_draw::PlaceCardOnTopOfDrawAction,
    game::Game,
    state::{GameState, Steps},
    step::Step,
};

pub struct ChooseCardInDiscardToPlaceOnTopOfDrawAction();

impl Action for ChooseCardInDiscardToPlaceOnTopOfDrawAction {
    fn run(&self, game: &mut Game) {
        match game.discard_pile.len() {
            0 => {}
            1 => game
                .action_queue
                .push_top(PlaceCardOnTopOfDrawAction(game.discard_pile.pop().unwrap())),
            _ => game
                .state
                .push_state(ChooseCardInDiscardToPlaceOnTopOfDrawGameState),
        }
    }
}

impl std::fmt::Debug for ChooseCardInDiscardToPlaceOnTopOfDrawAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "move one card in discard on top of draw")
    }
}

#[derive(Clone, Debug)]
struct ChooseCardInDiscardToPlaceOnTopOfDrawGameState;

impl GameState for ChooseCardInDiscardToPlaceOnTopOfDrawGameState {
    fn clone_box(&self) -> Box<dyn GameState> {
        Box::new(self.clone())
    }
    fn valid_steps(&self, game: &Game) -> Option<Steps> {
        let mut moves = Steps::default();
        for i in 0..game.discard_pile.len() {
            moves.push(PlaceCardInDiscardOnTopOfDrawStep { discard_index: i });
        }
        Some(moves)
    }
}

#[derive(Eq, PartialEq, Debug)]
pub struct PlaceCardInDiscardOnTopOfDrawStep {
    pub discard_index: usize,
}

impl Step for PlaceCardInDiscardOnTopOfDrawStep {
    fn should_pop_state(&self) -> bool {
        true
    }

    fn run(&self, game: &mut Game) {
        game.action_queue.push_top(PlaceCardOnTopOfDrawAction(
            game.discard_pile.remove(self.discard_index),
        ));
    }

    fn description(&self, game: &Game) -> String {
        format!(
            "place card on top of draw {} ({:?})",
            self.discard_index,
            game.discard_pile[self.discard_index].borrow()
        )
    }
}
