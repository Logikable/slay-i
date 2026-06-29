use crate::{
    action::Action,
    actions::exhaust_card::ExhaustCardAction,
    game::Game,
    state::{GameState, Steps},
    step::Step,
};

pub struct ChooseCardsInHandToExhaustAction(pub i32);

impl Action for ChooseCardsInHandToExhaustAction {
    fn run(&self, g: &mut Game) {
        if !g.hand.is_empty() {
            g.state.push_state(ChooseExhaustCardsInHandGameState {
                num_cards_remaining: self.0,
            });
        }
    }
}

impl std::fmt::Debug for ChooseCardsInHandToExhaustAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "purity {}", self.0)
    }
}

#[derive(Clone, Debug)]
struct ChooseExhaustCardsInHandGameState {
    pub num_cards_remaining: i32,
}

impl GameState for ChooseExhaustCardsInHandGameState {
    fn clone_box(&self) -> Box<dyn GameState> {
        Box::new(self.clone())
    }
    fn valid_steps(&self, game: &Game) -> Option<Steps> {
        let mut moves = Steps::default();
        moves.push(ChooseExhaustCardsInHandEndStep);
        if self.num_cards_remaining > 0 {
            for c in 0..game.hand.len() {
                moves.push(ChooseExhaustCardsInHandStep {
                    hand_index: c,
                    num_cards_remaining: self.num_cards_remaining,
                });
            }
        }
        Some(moves)
    }
}

#[derive(Eq, PartialEq, Debug)]
pub struct ChooseExhaustCardsInHandStep {
    pub hand_index: usize,
    pub num_cards_remaining: i32,
}

impl Step for ChooseExhaustCardsInHandStep {
    fn should_pop_state(&self) -> bool {
        true
    }

    fn run(&self, game: &mut Game) {
        game.chosen_cards.push(game.hand.remove(self.hand_index));
        game.state.push_state(ChooseExhaustCardsInHandGameState {
            num_cards_remaining: self.num_cards_remaining - 1,
        });
    }

    fn description(&self, game: &Game) -> String {
        format!(
            "exhaust {} ({:?})",
            self.hand_index,
            game.hand[self.hand_index].borrow()
        )
    }
}

#[derive(Eq, PartialEq, Debug)]
pub struct ChooseExhaustCardsInHandEndStep;

impl Step for ChooseExhaustCardsInHandEndStep {
    fn should_pop_state(&self) -> bool {
        true
    }

    fn run(&self, game: &mut Game) {
        while let Some(c) = game.chosen_cards.pop() {
            game.action_queue.push_top(ExhaustCardAction(c));
        }
    }

    fn description(&self, _: &Game) -> String {
        "end exhaust cards".to_owned()
    }
}
