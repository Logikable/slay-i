use crate::{
    action::Action,
    actions::upgrade::UpgradeAction,
    game::Game,
    state::{GameState, Steps},
    step::Step,
};

pub struct ArmamentsAction();

impl Action for ArmamentsAction {
    fn run(&self, game: &mut Game) {
        let upgradable = game
            .hand
            .iter()
            .filter(|c| c.borrow().can_upgrade())
            .collect::<Vec<_>>();
        match upgradable.len() {
            0 => {}
            1 => upgradable[0].borrow_mut().upgrade(),
            _ => game.state.push_state(ChooseArmamentsGameState),
        }
    }
}

impl std::fmt::Debug for ArmamentsAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "choose upgrade one card in hand")
    }
}

#[derive(Clone, Debug)]
struct ChooseArmamentsGameState;

impl GameState for ChooseArmamentsGameState {
    fn clone_box(&self) -> Box<dyn GameState> {
        Box::new(self.clone())
    }
    fn valid_steps(&self, game: &Game) -> Option<Steps> {
        let mut moves = Steps::default();
        for (i, c) in game.hand.iter().enumerate() {
            if c.borrow().can_upgrade() {
                moves.push(ArmamentsStep { hand_index: i });
            }
        }
        Some(moves)
    }
}

#[derive(Eq, PartialEq, Debug)]
pub struct ArmamentsStep {
    pub hand_index: usize,
}

impl Step for ArmamentsStep {
    fn should_pop_state(&self) -> bool {
        true
    }
    fn run(&self, game: &mut Game) {
        game.action_queue
            .push_top(UpgradeAction(game.hand[self.hand_index].clone()));
    }

    fn description(&self, game: &Game) -> String {
        format!(
            "upgrade card {} ({:?})",
            self.hand_index,
            game.hand[self.hand_index].borrow()
        )
    }
}
