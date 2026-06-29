use crate::{
    action::Action,
    actions::place_card_in_hand::PlaceCardInHandAction,
    cards::CardClass,
    game::Game,
    state::{GameState, Steps},
    step::Step,
};

pub struct ExhumeAction();

enum Count {
    Zero,
    One(usize),
    Many,
}

impl Action for ExhumeAction {
    fn run(&self, game: &mut Game) {
        if game.hand_is_full() {
            return;
        }
        let mut count = Count::Zero;
        for (i, c) in game.exhaust_pile.iter().enumerate() {
            if c.borrow().class != CardClass::Exhume {
                match count {
                    Count::Zero => count = Count::One(i),
                    Count::One(_) => {
                        count = Count::Many;
                        break;
                    }
                    Count::Many => unreachable!(),
                }
            }
        }
        match count {
            Count::Zero => {}
            Count::One(i) => game.hand.push(game.exhaust_pile.remove(i)),
            Count::Many => game.state.push_state(ChooseExhumeGameState),
        }
    }
}

impl std::fmt::Debug for ExhumeAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "exhume")
    }
}

#[derive(Clone, Debug)]
struct ChooseExhumeGameState;

impl GameState for ChooseExhumeGameState {
    fn clone_box(&self) -> Box<dyn GameState> {
        Box::new(self.clone())
    }
    fn valid_steps(&self, game: &Game) -> Option<Steps> {
        let mut moves = Steps::default();
        for (i, c) in game.exhaust_pile.iter().enumerate() {
            if c.borrow().class != CardClass::Exhume {
                moves.push(ExhumeStep { exhaust_index: i });
            }
        }
        Some(moves)
    }
}

#[derive(Eq, PartialEq, Debug)]
pub struct ExhumeStep {
    pub exhaust_index: usize,
}

impl Step for ExhumeStep {
    fn should_pop_state(&self) -> bool {
        true
    }

    fn run(&self, game: &mut Game) {
        game.action_queue.push_top(PlaceCardInHandAction(
            game.exhaust_pile.remove(self.exhaust_index),
        ));
    }

    fn description(&self, game: &Game) -> String {
        format!(
            "exhaust {} ({:?})",
            self.exhaust_index,
            game.exhaust_pile[self.exhaust_index].borrow()
        )
    }
}
