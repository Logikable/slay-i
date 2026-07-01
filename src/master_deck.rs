use crate::{
    actions::{
        add_card_to_master_deck::AddCardToMasterDeckAction,
        removed_card_from_master_deck::RemovedCardFromMasterDeckAction, upgrade::UpgradeAction,
    },
    cards::{CardType, transformed},
    game::{Game, RunActionsGameState},
    state::{GameState, Steps},
    step::Step,
};

#[derive(Debug)]
pub struct ChooseTransformMasterGameState {
    pub num_cards_remaining: usize,
    pub upgrade: bool,
}

impl GameState for ChooseTransformMasterGameState {
    fn run(&self, game: &mut Game) {
        let count = game
            .master_deck
            .iter()
            .filter(|c| c.borrow().can_remove_from_master_deck())
            .count();
        if count <= self.num_cards_remaining {
            for i in (0..game.master_deck.len()).rev() {
                if game.master_deck[i].borrow().can_remove_from_master_deck() {
                    let c = game.master_deck.remove(i);
                    game.chosen_cards.push(c);
                }
            }
            game.state.push_state(TransformChosenCardsGameState {
                upgrade: self.upgrade,
            });
        }
    }
    fn valid_steps(&self, game: &Game) -> Option<Steps> {
        let mut moves = Steps::default();
        for (i, c) in game.master_deck.iter().enumerate() {
            if c.borrow().can_remove_from_master_deck() {
                moves.push(ChooseTransformMasterStep {
                    master_index: i,
                    num_cards_remaining: self.num_cards_remaining,
                    upgrade: self.upgrade,
                });
            }
        }
        if moves.steps.is_empty() {
            None
        } else {
            Some(moves)
        }
    }
}

#[derive(Eq, PartialEq, Debug)]
pub struct ChooseTransformMasterStep {
    pub master_index: usize,
    pub num_cards_remaining: usize,
    pub upgrade: bool,
}

impl Step for ChooseTransformMasterStep {
    fn should_pop_state(&self) -> bool {
        true
    }

    fn run(&self, game: &mut Game) {
        let c = game.master_deck.remove(self.master_index);
        game.chosen_cards.push(c);
        let num_cards_remaining = self.num_cards_remaining - 1;
        if num_cards_remaining == 0 {
            game.state.push_state(TransformChosenCardsGameState {
                upgrade: self.upgrade,
            });
        } else {
            game.state.push_state(ChooseTransformMasterGameState {
                num_cards_remaining,
                upgrade: self.upgrade,
            });
        }
    }

    fn description(&self, game: &Game) -> String {
        format!(
            "transform {:?}",
            game.master_deck[self.master_index].borrow()
        )
    }
}

#[derive(Debug)]
pub struct TransformChosenCardsGameState {
    pub upgrade: bool,
}

impl GameState for TransformChosenCardsGameState {
    fn run(&self, game: &mut Game) {
        while let Some(c) = game.chosen_cards.pop() {
            let class = c.borrow().class;
            let transformed = transformed(class, &mut game.rng);
            game.action_queue
                .push_bot(RemovedCardFromMasterDeckAction(class));

            let c = if self.upgrade {
                game.new_card_upgraded(transformed)
            } else {
                game.new_card(transformed)
            };
            game.action_queue.push_bot(AddCardToMasterDeckAction(c));
        }
        game.state.push_state(RunActionsGameState);
    }
}

#[derive(Debug)]
pub struct ChooseUpgradeMasterGameState;

impl GameState for ChooseUpgradeMasterGameState {
    fn valid_steps(&self, game: &Game) -> Option<Steps> {
        let mut moves = Steps::default();
        for (i, c) in game.master_deck.iter().enumerate() {
            if c.borrow().can_upgrade() {
                moves.push(ChooseUpgradeMasterStep { master_index: i });
            }
        }
        Some(moves)
    }
}

#[derive(Eq, PartialEq, Debug)]
pub struct ChooseUpgradeMasterStep {
    pub master_index: usize,
}

impl Step for ChooseUpgradeMasterStep {
    fn should_pop_state(&self) -> bool {
        true
    }

    fn run(&self, game: &mut Game) {
        let c = game.master_deck[self.master_index].clone();
        game.action_queue.push_bot(UpgradeAction(c));
        game.state.push_state(RunActionsGameState);
    }

    fn description(&self, game: &Game) -> String {
        format!("upgrade {:?}", game.master_deck[self.master_index].borrow())
    }
}

#[derive(Debug)]
pub struct ChooseRemoveFromMasterGameState {
    pub num_cards_remaining: usize,
}

impl GameState for ChooseRemoveFromMasterGameState {
    fn run(&self, game: &mut Game) {
        let count = game
            .master_deck
            .iter()
            .filter(|c| c.borrow().can_remove_from_master_deck())
            .count();
        if count <= self.num_cards_remaining {
            for i in (0..game.master_deck.len()).rev() {
                if game.master_deck[i].borrow().can_remove_from_master_deck() {
                    let c = game.master_deck.remove(i);
                    game.chosen_cards.push(c);
                }
            }
            game.state.push_state(RemoveChosenCardsGameState);
        }
    }
    fn valid_steps(&self, game: &Game) -> Option<Steps> {
        let mut moves = Steps::default();
        for (i, c) in game.master_deck.iter().enumerate() {
            if c.borrow().can_remove_from_master_deck() {
                moves.push(ChooseRemoveFromMasterStep {
                    master_index: i,
                    num_cards_remaining: self.num_cards_remaining,
                });
            }
        }
        if moves.steps.is_empty() {
            None
        } else {
            Some(moves)
        }
    }
}

#[derive(Eq, PartialEq, Debug)]
pub struct ChooseRemoveFromMasterStep {
    pub master_index: usize,
    pub num_cards_remaining: usize,
}

impl Step for ChooseRemoveFromMasterStep {
    fn should_pop_state(&self) -> bool {
        true
    }

    fn run(&self, game: &mut Game) {
        game.chosen_cards
            .push(game.master_deck.remove(self.master_index));
        let num_cards_remaining = self.num_cards_remaining - 1;
        if num_cards_remaining == 0 {
            game.state.push_state(RemoveChosenCardsGameState);
        } else {
            game.state.push_state(ChooseRemoveFromMasterGameState {
                num_cards_remaining,
            });
        }
    }

    fn description(&self, game: &Game) -> String {
        format!("remove {:?}", game.master_deck[self.master_index].borrow())
    }
}

#[derive(Debug)]
pub struct RemoveChosenCardsGameState;

impl GameState for RemoveChosenCardsGameState {
    fn run(&self, game: &mut Game) {
        while let Some(c) = game.chosen_cards.pop() {
            game.action_queue
                .push_bot(RemovedCardFromMasterDeckAction(c.borrow().class));
        }
        game.state.push_state(RunActionsGameState);
    }
}

#[derive(Debug)]
pub struct ChooseDuplicateCardInMasterGameState;

impl GameState for ChooseDuplicateCardInMasterGameState {
    fn valid_steps(&self, game: &Game) -> Option<Steps> {
        let mut moves = Steps::default();
        for i in 0..game.master_deck.len() {
            moves.push(DuplicateCardInMasterStep { master_index: i });
        }
        Some(moves)
    }
}

#[derive(Eq, PartialEq, Debug)]
pub struct DuplicateCardInMasterStep {
    pub master_index: usize,
}

impl Step for DuplicateCardInMasterStep {
    fn should_pop_state(&self) -> bool {
        true
    }

    fn run(&self, game: &mut Game) {
        let original = game.master_deck[self.master_index].clone();
        let c = game.clone_card_ref_new_id(&original);
        c.borrow_mut().is_bottled = false;
        game.action_queue.push_bot(AddCardToMasterDeckAction(c));
        game.state.push_state(RunActionsGameState);
    }

    fn description(&self, game: &Game) -> String {
        format!(
            "duplicate {:?}",
            game.master_deck[self.master_index].borrow()
        )
    }
}

#[derive(Debug)]
pub struct ChooseBottledCardGameState {
    pub ty: CardType,
}

impl GameState for ChooseBottledCardGameState {
    fn valid_steps(&self, game: &Game) -> Option<Steps> {
        let mut steps = Steps::default();
        for (i, c) in game.master_deck.iter().enumerate() {
            if c.borrow().class.ty() == self.ty {
                steps.push(ChooseBottledCardStep { master_index: i });
            }
        }
        Some(steps)
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct ChooseBottledCardStep {
    pub master_index: usize,
}

impl Step for ChooseBottledCardStep {
    fn should_pop_state(&self) -> bool {
        true
    }
    fn run(&self, game: &mut Game) {
        game.master_deck[self.master_index].borrow_mut().is_bottled = true;
    }
    fn description(&self, game: &Game) -> String {
        format!("bottle {:?}", game.master_deck[self.master_index].borrow())
    }
}
