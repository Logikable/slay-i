use crate::{
    actions::{block::BlockAction, damage::DamageAction, gain_status::GainStatusAction},
    creature::Creature,
    game::{CreatureRef, Rand},
    monster::{Intent, MonsterBehavior, MonsterInfo},
    move_history::MoveHistory,
    queue::ActionQueue,
    status::Status,
};

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
enum Action {
    Start,
    EventStart,
    Sleep1,
    Sleep2,
    Sleep3,
    Attack,
    Debuff,
    Stunned,
}

#[derive(Clone)]
pub struct Lagavulin {
    action: Action,
    history: MoveHistory<Action>,
}

impl Lagavulin {
    pub fn new() -> Self {
        Self {
            action: Action::Start,
            history: MoveHistory::new(),
        }
    }
    pub fn new_event() -> Self {
        Self {
            action: Action::EventStart,
            history: MoveHistory::new(),
        }
    }
}

impl MonsterBehavior for Lagavulin {
    fn clone_box(&self) -> Box<dyn MonsterBehavior> {
        Box::new(self.clone())
    }
    fn name(&self) -> &'static str {
        "lagavulin"
    }

    fn hp_range(&self) -> (i32, i32) {
        (112, 115)
    }

    fn pre_combat(&self, queue: &mut ActionQueue, this: CreatureRef, _rng: &mut Rand) {
        queue.push_bot(BlockAction::monster(this, 8));
        queue.push_bot(GainStatusAction {
            status: Status::Metallicize,
            amount: 8,
            target: this,
        });
    }

    fn on_take_damage(&mut self, _this: CreatureRef, _this_creature: &mut Creature) {
        if matches!(
            self.action,
            Action::Sleep1 | Action::Sleep2 | Action::Sleep3
        ) {
            self.action = Action::Stunned;
        }
    }

    fn take_turn(&mut self, this: CreatureRef, queue: &mut ActionQueue, _: &MonsterInfo) {
        match self.action {
            Action::Sleep1 | Action::Sleep2 | Action::Sleep3 | Action::Stunned => {}
            Action::Attack => {
                queue.push_bot(DamageAction::from_monster(20, this));
            }
            Action::Debuff => {
                queue.push_bot(GainStatusAction {
                    status: Status::Dexterity,
                    amount: -2,
                    target: CreatureRef::player(),
                });
                queue.push_bot(GainStatusAction {
                    status: Status::Strength,
                    amount: -2,
                    target: CreatureRef::player(),
                });
            }
            Action::Start | Action::EventStart => unreachable!(),
        }
    }

    fn roll_next_action(&mut self, _rng: &mut Rand, _info: &MonsterInfo) {
        let next = match self.action {
            Action::EventStart => Action::Debuff,
            Action::Start => Action::Sleep1,
            Action::Sleep1 => Action::Sleep2,
            Action::Sleep2 => Action::Sleep3,
            Action::Sleep3 => Action::Attack,
            Action::Attack => {
                if self.history.last_two(Action::Attack) {
                    Action::Debuff
                } else {
                    Action::Attack
                }
            }
            Action::Debuff | Action::Stunned => Action::Attack,
        };
        self.action = next;
        self.history.add(next);
    }

    fn get_intent(&self) -> Intent {
        match self.action {
            Action::Start | Action::EventStart => panic!(),
            Action::Sleep1 | Action::Sleep2 | Action::Sleep3 => Intent::Sleep,
            Action::Stunned => Intent::Stun,
            Action::Attack => Intent::Attack(20, 1),
            Action::Debuff => Intent::StrongDebuff,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        actions::increase_max_hp::IncreaseMaxHPAction, assert_matches, cards::CardClass,
        combat::EndTurnStep, game::GameBuilder,
    };

    #[test]
    fn test_lagavulin_wake() {
        let mut g = GameBuilder::default().build_combat_with_monster(Lagavulin::new());
        g.run_action(IncreaseMaxHPAction(1000));
        assert_matches!(g.monsters[0].behavior.get_intent(), Intent::Sleep);
        g.step_test(EndTurnStep);
        assert_matches!(g.monsters[0].behavior.get_intent(), Intent::Sleep);
        g.step_test(EndTurnStep);
        assert_matches!(g.monsters[0].behavior.get_intent(), Intent::Sleep);
        g.step_test(EndTurnStep);
        assert_matches!(g.monsters[0].behavior.get_intent(), Intent::Attack(20, 1));
        g.step_test(EndTurnStep);
        assert_matches!(g.monsters[0].behavior.get_intent(), Intent::Attack(20, 1));
        g.step_test(EndTurnStep);
        assert_matches!(g.monsters[0].behavior.get_intent(), Intent::StrongDebuff);
        g.step_test(EndTurnStep);
        assert_eq!(g.player.get_status(Status::Dexterity), Some(-2));
        assert_eq!(g.player.get_status(Status::Strength), Some(-2));
        assert_matches!(g.monsters[0].behavior.get_intent(), Intent::Attack(20, 1));
        g.step_test(EndTurnStep);
        assert_matches!(g.monsters[0].behavior.get_intent(), Intent::Attack(20, 1));
        g.step_test(EndTurnStep);
        assert_matches!(g.monsters[0].behavior.get_intent(), Intent::StrongDebuff);
        g.player.set_status(Status::Artifact, 1);
        g.step_test(EndTurnStep);
        assert_eq!(g.player.get_status(Status::Dexterity), Some(-2));
        assert_eq!(g.player.get_status(Status::Strength), Some(-4));
        assert_matches!(g.monsters[0].behavior.get_intent(), Intent::Attack(20, 1));
        g.step_test(EndTurnStep);
        assert_matches!(g.monsters[0].behavior.get_intent(), Intent::Attack(20, 1));
        g.step_test(EndTurnStep);
        assert_matches!(g.monsters[0].behavior.get_intent(), Intent::StrongDebuff);
    }

    #[test]
    fn test_lagavulin_elite() {
        let mut g = GameBuilder::default().build_combat_with_monster(Lagavulin::new_event());
        g.run_action(IncreaseMaxHPAction(1000));
        assert_matches!(g.monsters[0].behavior.get_intent(), Intent::StrongDebuff);
        g.step_test(EndTurnStep);
        assert_matches!(g.monsters[0].behavior.get_intent(), Intent::Attack(20, 1));
        g.step_test(EndTurnStep);
        assert_matches!(g.monsters[0].behavior.get_intent(), Intent::Attack(20, 1));
        g.step_test(EndTurnStep);
        assert_matches!(g.monsters[0].behavior.get_intent(), Intent::StrongDebuff);
        g.step_test(EndTurnStep);
        assert_matches!(g.monsters[0].behavior.get_intent(), Intent::Attack(20, 1));
        g.step_test(EndTurnStep);
        assert_matches!(g.monsters[0].behavior.get_intent(), Intent::Attack(20, 1));
        g.step_test(EndTurnStep);
        assert_matches!(g.monsters[0].behavior.get_intent(), Intent::StrongDebuff);
        g.step_test(EndTurnStep);
        assert_matches!(g.monsters[0].behavior.get_intent(), Intent::Attack(20, 1));
        g.step_test(EndTurnStep);
        assert_matches!(g.monsters[0].behavior.get_intent(), Intent::Attack(20, 1));
        g.step_test(EndTurnStep);
        assert_matches!(g.monsters[0].behavior.get_intent(), Intent::StrongDebuff);
    }

    #[test]
    fn test_lagavulin_force_wake_1() {
        let mut g = GameBuilder::default().build_combat_with_monster(Lagavulin::new());
        g.run_action(IncreaseMaxHPAction(1000));
        assert_matches!(g.monsters[0].behavior.get_intent(), Intent::Sleep);
        g.play_card(CardClass::Bash, Some(CreatureRef::monster(0)));
        assert_matches!(g.monsters[0].behavior.get_intent(), Intent::Sleep);
        g.play_card(CardClass::SwiftStrike, Some(CreatureRef::monster(0)));
        assert_matches!(g.monsters[0].behavior.get_intent(), Intent::Stun);
        g.step_test(EndTurnStep);
        assert_matches!(g.monsters[0].behavior.get_intent(), Intent::Attack(20, 1));
        g.step_test(EndTurnStep);
        assert_matches!(g.monsters[0].behavior.get_intent(), Intent::Attack(20, 1));
        g.step_test(EndTurnStep);
        assert_matches!(g.monsters[0].behavior.get_intent(), Intent::StrongDebuff);
    }

    #[test]
    fn test_lagavulin_force_wake_2() {
        let mut g = GameBuilder::default().build_combat_with_monster(Lagavulin::new());
        g.run_action(IncreaseMaxHPAction(1000));
        assert_matches!(g.monsters[0].behavior.get_intent(), Intent::Sleep);
        g.step_test(EndTurnStep);
        assert_matches!(g.monsters[0].behavior.get_intent(), Intent::Sleep);
        g.play_card(CardClass::Bash, Some(CreatureRef::monster(0)));
        assert_matches!(g.monsters[0].behavior.get_intent(), Intent::Sleep);
        g.play_card(CardClass::SwiftStrike, Some(CreatureRef::monster(0)));
        assert_matches!(g.monsters[0].behavior.get_intent(), Intent::Stun);
        g.step_test(EndTurnStep);
        assert_matches!(g.monsters[0].behavior.get_intent(), Intent::Attack(20, 1));
        g.step_test(EndTurnStep);
        assert_matches!(g.monsters[0].behavior.get_intent(), Intent::Attack(20, 1));
        g.step_test(EndTurnStep);
        assert_matches!(g.monsters[0].behavior.get_intent(), Intent::StrongDebuff);
    }
}
