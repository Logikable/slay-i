use crate::{game::Game, step::Step};

use std::fmt::Debug;

#[derive(Default, Debug)]
pub struct Steps {
    pub steps: Vec<Box<dyn Step>>,
}

impl Steps {
    pub fn push<T: Step>(&mut self, step: T) {
        self.steps.push(Box::new(step));
    }
}

pub trait GameState: Debug {
    fn run(&self, _: &mut Game) {}
    fn valid_steps(&self, _: &Game) -> Option<Steps> {
        None
    }
    // Forks this state for `Game::clone_for_search`. Only states that can sit
    // on the stack while combat awaits player input need to implement this;
    // the rest panic so an unsupported search entry point fails loudly.
    fn clone_box(&self) -> Box<dyn GameState> {
        panic!("clone_box not implemented for GameState {self:?}")
    }
}

impl Clone for Box<dyn GameState> {
    fn clone(&self) -> Self {
        self.clone_box()
    }
}

#[derive(Eq, PartialEq, Debug)]
pub struct ContinueStep;

impl Step for ContinueStep {
    fn should_pop_state(&self) -> bool {
        true
    }
    fn run(&self, _: &mut Game) {}

    fn description(&self, _: &Game) -> String {
        "continue".to_owned()
    }
}

#[derive(Default)]
pub struct GameStateManager {
    stack: Vec<Box<dyn GameState>>,
    debug: bool,
}

impl std::fmt::Debug for GameStateManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "state stack: {:?}", self.stack)
    }
}

impl GameStateManager {
    pub fn clear(&mut self) {
        self.stack.clear();
    }
    pub fn is_empty(&self) -> bool {
        self.stack.is_empty()
    }
    pub fn set_debug(&mut self) {
        self.debug = true;
    }
    pub fn push_state<T: GameState + 'static>(&mut self, state: T) {
        self.push_boxed_state(Box::new(state));
    }
    pub fn push_boxed_state(&mut self, state: Box<dyn GameState>) {
        if self.debug {
            println!("push_state {:?}", state);
        }
        self.stack.push(state);
    }
    pub fn pop_state(&mut self) -> Option<Box<dyn GameState>> {
        let state = self.stack.pop();
        if self.debug
            && let Some(s) = &state
        {
            println!("pop_state {:?} ({:?})", s, self.stack);
        }
        state
    }
    pub fn peek(&self) -> &dyn GameState {
        self.stack.last().unwrap().as_ref()
    }
    pub fn clone_for_search(&self) -> GameStateManager {
        GameStateManager {
            stack: self.stack.iter().map(|s| s.clone_box()).collect(),
            debug: self.debug,
        }
    }
}
