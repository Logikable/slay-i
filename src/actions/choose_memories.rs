use crate::{
    action::Action,
    actions::memories::MemoriesAction,
    game::Game,
    state::{GameState, Steps},
    step::Step,
};

pub struct ChooseMemoriesAction(pub i32);

impl Action for ChooseMemoriesAction {
    fn run(&self, game: &mut Game) {
        if game.discard_pile.len() as i32 <= self.0 {
            let count =
                (game.discard_pile.len() as i32).min(Game::MAX_HAND_SIZE - game.hand.len() as i32);
            for _ in 0..count {
                game.action_queue
                    .push_top(MemoriesAction(game.discard_pile.remove(0)));
            }
        } else {
            game.state.push_state(ChooseMemoriesGameState {
                num_cards_remaining: self.0.min(Game::MAX_HAND_SIZE - game.hand.len() as i32),
            });
        }
    }
}

impl std::fmt::Debug for ChooseMemoriesAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "choose {} card(s) to memories", self.0)
    }
}

#[derive(Clone, Debug)]
struct ChooseMemoriesGameState {
    num_cards_remaining: i32,
}

impl GameState for ChooseMemoriesGameState {
    fn clone_box(&self) -> Box<dyn GameState> {
        Box::new(self.clone())
    }
    fn valid_steps(&self, game: &Game) -> Option<Steps> {
        let mut moves = Steps::default();
        moves.push(ChooseMemoriesEndStep);
        if self.num_cards_remaining != 0 {
            for c in 0..game.discard_pile.len() {
                moves.push(ChooseMemoriesStep {
                    discard_index: c,
                    num_cards_remaining: self.num_cards_remaining,
                });
            }
        }
        Some(moves)
    }
}

#[derive(Eq, PartialEq, Debug)]
pub struct ChooseMemoriesStep {
    pub discard_index: usize,
    pub num_cards_remaining: i32,
}

impl Step for ChooseMemoriesStep {
    fn should_pop_state(&self) -> bool {
        true
    }

    fn run(&self, game: &mut Game) {
        game.chosen_cards
            .push(game.discard_pile.remove(self.discard_index));
        if self.num_cards_remaining == 0 {
            while let Some(c) = game.chosen_cards.pop() {
                game.action_queue.push_top(MemoriesAction(c));
            }
        } else {
            game.state.push_state(ChooseMemoriesGameState {
                num_cards_remaining: self.num_cards_remaining - 1,
            });
        }
    }

    fn description(&self, game: &Game) -> String {
        format!(
            "memories {} ({:?})",
            self.discard_index,
            game.discard_pile[self.discard_index].borrow()
        )
    }
}

#[derive(Eq, PartialEq, Debug)]
pub struct ChooseMemoriesEndStep;

impl Step for ChooseMemoriesEndStep {
    fn should_pop_state(&self) -> bool {
        true
    }

    fn run(&self, game: &mut Game) {
        while let Some(c) = game.chosen_cards.pop() {
            game.action_queue.push_top(MemoriesAction(c));
        }
    }

    fn description(&self, _: &Game) -> String {
        "end memories".to_owned()
    }
}
