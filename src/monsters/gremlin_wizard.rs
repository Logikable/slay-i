use crate::{
    actions::damage::DamageAction,
    game::{CreatureRef, Rand},
    monster::{Intent, MonsterBehavior, MonsterInfo},
    queue::ActionQueue,
};

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
enum Action {
    Charge,
    Attack,
}

#[derive(Clone)]
pub struct GremlinWizard {
    current_charge: i32,
    action: Action,
}

impl GremlinWizard {
    pub fn new() -> Self {
        Self {
            current_charge: 0,
            action: Action::Charge,
        }
    }
}

impl MonsterBehavior for GremlinWizard {
    fn clone_box(&self) -> Box<dyn MonsterBehavior> {
        Box::new(self.clone())
    }
    fn name(&self) -> &'static str {
        "gremlin wizard"
    }

    fn hp_range(&self) -> (i32, i32) {
        (22, 26)
    }

    fn take_turn(&mut self, this: CreatureRef, queue: &mut ActionQueue, _: &MonsterInfo) {
        match self.action {
            Action::Charge => {
                self.current_charge += 1;
            }
            Action::Attack => {
                queue.push_bot(DamageAction::from_monster(30, this));
            }
        }
    }

    fn roll_next_action(&mut self, _r: &mut Rand, _info: &MonsterInfo) {
        let next = if self.current_charge < 2 {
            Action::Charge
        } else {
            Action::Attack
        };

        self.action = next;
    }

    fn get_intent(&self) -> Intent {
        match self.action {
            Action::Charge => Intent::Buff,
            Action::Attack => Intent::Attack(30, 1),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{assert_matches, combat::EndTurnStep, game::GameBuilder};

    #[test]
    fn test_gremlin_wizard_logic() {
        let mut g = GameBuilder::default().build_combat_with_monster(GremlinWizard::new());

        assert_matches!(g.monsters[0].behavior.get_intent(), Intent::Buff);

        g.step_test(EndTurnStep);
        assert_matches!(g.monsters[0].behavior.get_intent(), Intent::Buff);

        g.step_test(EndTurnStep);
        assert_matches!(g.monsters[0].behavior.get_intent(), Intent::Attack(30, 1));

        g.step_test(EndTurnStep);
        assert_matches!(g.monsters[0].behavior.get_intent(), Intent::Attack(30, 1));
    }
}
