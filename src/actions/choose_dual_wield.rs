use crate::{
    action::Action,
    actions::dual_wield::DualWieldAction,
    card::CardRef,
    cards::CardType,
    game::Game,
    state::{GameState, Steps},
    step::Step,
};

pub struct ChooseDualWieldAction(pub i32);

enum Count {
    Zero,
    One(usize),
    Many,
}

pub fn can_dual_wield(c: &CardRef) -> bool {
    matches!(c.borrow().class.ty(), CardType::Attack | CardType::Power)
}

impl Action for ChooseDualWieldAction {
    fn run(&self, game: &mut Game) {
        let mut count = Count::Zero;
        for (i, c) in game.hand.iter().enumerate() {
            if can_dual_wield(c) {
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
            Count::One(i) => {
                game.action_queue.push_top(DualWieldAction {
                    card: game.hand.remove(i),
                    amount: self.0,
                    destroy_original: false,
                });
            }
            Count::Many => game.state.push_state(DualWieldGameState { amount: self.0 }),
        }
    }
}

impl std::fmt::Debug for ChooseDualWieldAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "choose dual wield")
    }
}

#[derive(Eq, PartialEq, Debug)]
pub struct DualWieldStep {
    pub hand_index: usize,
    pub amount: i32,
}

impl Step for DualWieldStep {
    fn should_pop_state(&self) -> bool {
        true
    }

    fn run(&self, game: &mut Game) {
        game.action_queue.push_top(DualWieldAction {
            card: game.hand.remove(self.hand_index),
            amount: self.amount,
            destroy_original: true,
        });
    }

    fn description(&self, game: &Game) -> String {
        format!(
            "dual wield {} ({:?})",
            self.hand_index,
            game.hand[self.hand_index].borrow()
        )
    }
}

#[derive(Clone, Debug)]
struct DualWieldGameState {
    amount: i32,
}

impl GameState for DualWieldGameState {
    fn clone_box(&self) -> Box<dyn GameState> {
        Box::new(self.clone())
    }
    fn valid_steps(&self, game: &Game) -> Option<Steps> {
        let mut moves = Steps::default();
        for (i, c) in game.hand.iter().enumerate() {
            if can_dual_wield(c) {
                moves.push(DualWieldStep {
                    hand_index: i,
                    amount: self.amount,
                });
            }
        }
        Some(moves)
    }
}
