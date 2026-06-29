use crate::{
    action::Action,
    actions::place_card_in_hand::PlaceCardInHandAction,
    cards::CardType,
    game::Game,
    state::{GameState, Steps},
    step::Step,
};

pub struct ChooseCardInDrawToPlaceInHandAction(pub CardType);

enum Count {
    Zero,
    One(usize),
    Many,
}

impl Action for ChooseCardInDrawToPlaceInHandAction {
    fn run(&self, game: &mut Game) {
        let mut count = Count::Zero;
        for (i, c) in game.draw_pile.get_all().into_iter().enumerate() {
            if c.borrow().class.ty() == self.0 {
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
            Count::Zero => unreachable!(),
            Count::One(i) => {
                let c = game.draw_pile.take(i);
                game.action_queue.push_top(PlaceCardInHandAction(c));
            }
            Count::Many => game.state.push_state(FetchCardFromDrawGameState(self.0)),
        }
    }
}

impl std::fmt::Debug for ChooseCardInDrawToPlaceInHandAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "choose card in draw to place in hand ({:?})", self.0)
    }
}

#[derive(Clone, Debug)]
struct FetchCardFromDrawGameState(CardType);

impl GameState for FetchCardFromDrawGameState {
    fn clone_box(&self) -> Box<dyn GameState> {
        Box::new(self.clone())
    }
    fn valid_steps(&self, game: &Game) -> Option<Steps> {
        let mut moves = Steps::default();
        for (i, c) in game.draw_pile.get_all().iter().enumerate() {
            if c.borrow().class.ty() == self.0 {
                moves.push(FetchFromDrawStep { draw_index: i });
            }
        }
        Some(moves)
    }

    fn run(&self, _: &mut Game) {}
}

#[derive(Eq, PartialEq, Debug)]
pub struct FetchFromDrawStep {
    pub draw_index: usize,
}

impl Step for FetchFromDrawStep {
    fn should_pop_state(&self) -> bool {
        true
    }

    fn run(&self, game: &mut Game) {
        let c = game.draw_pile.take(self.draw_index);
        game.action_queue.push_top(PlaceCardInHandAction(c));
    }

    fn description(&self, game: &Game) -> String {
        format!(
            "fetch {} ({:?})",
            self.draw_index,
            game.draw_pile.get(self.draw_index).borrow()
        )
    }
}
