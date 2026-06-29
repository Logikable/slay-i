use crate::{
    actions::{
        block::BlockAction, damage::DamageAction, gain_status::GainStatusAction,
        remove_status::RemoveStatusAction,
    },
    creature::Creature,
    game::{CreatureRef, Rand},
    monster::{Intent, MonsterBehavior, MonsterInfo},
    queue::ActionQueue,
    status::Status,
};

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
enum Action {
    ChargingUp,
    FierceBash,
    VentSteam,
    Whirlwind,
    DefensiveMode,
    RollAttack,
    TwinSlam,
}

#[derive(Clone)]
pub struct Guardian {
    action: Action,
    mode_shift_amount: i32,
}

impl Guardian {
    pub fn new() -> Self {
        Self {
            action: Action::ChargingUp,
            mode_shift_amount: 40,
        }
    }

    fn apply_mode_shift(&self, this: CreatureRef, queue: &mut ActionQueue) {
        queue.push_bot(GainStatusAction {
            status: Status::ModeShift,
            amount: self.mode_shift_amount,
            target: this,
        });
    }
}

impl MonsterBehavior for Guardian {
    fn clone_box(&self) -> Box<dyn MonsterBehavior> {
        Box::new(self.clone())
    }
    fn name(&self) -> &'static str {
        "guardian"
    }

    fn hp_range(&self) -> (i32, i32) {
        (250, 250)
    }

    fn pre_combat(&self, queue: &mut ActionQueue, this: CreatureRef, _: &mut Rand) {
        self.apply_mode_shift(this, queue);
    }

    fn on_take_damage(&mut self, _this: CreatureRef, c: &mut Creature) {
        if let Some(amount) = c.get_status(Status::ModeShift)
            && amount <= 0
        {
            self.action = Action::DefensiveMode;
            c.remove_status(Status::ModeShift);
        }
    }

    fn take_turn(&mut self, this: CreatureRef, queue: &mut ActionQueue, _: &MonsterInfo) {
        self.action = match self.action {
            Action::ChargingUp => {
                queue.push_bot(BlockAction::monster(this, 9));
                Action::FierceBash
            }
            Action::FierceBash => {
                queue.push_bot(DamageAction::from_monster(36, this));
                Action::VentSteam
            }
            Action::VentSteam => {
                queue.push_bot(GainStatusAction {
                    status: Status::Weak,
                    amount: 2,
                    target: CreatureRef::player(),
                });
                queue.push_bot(GainStatusAction {
                    status: Status::Vulnerable,
                    amount: 2,
                    target: CreatureRef::player(),
                });
                Action::Whirlwind
            }
            Action::Whirlwind => {
                for _ in 0..4 {
                    queue.push_bot(DamageAction::from_monster(5, this));
                }
                Action::ChargingUp
            }
            Action::DefensiveMode => {
                queue.push_bot(GainStatusAction {
                    status: Status::SharpHide,
                    amount: 4,
                    target: this,
                });
                Action::RollAttack
            }
            Action::RollAttack => {
                queue.push_bot(DamageAction::from_monster(10, this));
                Action::TwinSlam
            }
            Action::TwinSlam => {
                for _ in 0..2 {
                    queue.push_bot(DamageAction::from_monster(8, this));
                }
                queue.push_bot(RemoveStatusAction {
                    status: Status::SharpHide,
                    target: this,
                });
                self.mode_shift_amount += 10;
                self.apply_mode_shift(this, queue);
                Action::Whirlwind
            }
        };
    }

    fn roll_next_action(&mut self, _: &mut Rand, _info: &MonsterInfo) {}

    fn get_intent(&self) -> Intent {
        match self.action {
            Action::ChargingUp => Intent::Defend,
            Action::FierceBash => Intent::Attack(36, 1),
            Action::VentSteam => Intent::StrongDebuff,
            Action::Whirlwind => Intent::Attack(5, 4),
            Action::DefensiveMode => Intent::Buff,
            Action::RollAttack => Intent::Attack(10, 1),
            Action::TwinSlam => Intent::Attack(8, 2),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        actions::increase_max_hp::IncreaseMaxHPAction, assert_matches, combat::EndTurnStep,
        game::GameBuilder, potion::Potion,
    };

    use super::*;

    #[test]
    fn test_basic() {
        let mut g = GameBuilder::default().build_combat_with_monster(Guardian::new());
        g.run_action(IncreaseMaxHPAction(999));

        assert_matches!(g.monsters[0].behavior.get_intent(), Intent::Defend);
        g.step_test(EndTurnStep);
        assert_eq!(g.monsters[0].creature.block, 9);

        assert_matches!(g.monsters[0].behavior.get_intent(), Intent::Attack(36, 1));
        g.step_test(EndTurnStep);

        assert_matches!(g.monsters[0].behavior.get_intent(), Intent::StrongDebuff);
        g.step_test(EndTurnStep);
        assert_eq!(g.player.get_status(Status::Weak), Some(2));
        assert_eq!(g.player.get_status(Status::Vulnerable), Some(2));

        assert_matches!(g.monsters[0].behavior.get_intent(), Intent::Attack(5, 4));
        g.player.cur_hp = 500;
        g.step_test(EndTurnStep);
        assert_eq!(g.player.cur_hp, 500 - 28);

        assert_matches!(g.monsters[0].behavior.get_intent(), Intent::Defend);
    }

    #[test]
    fn test_mode_shift() {
        let mut g = GameBuilder::default().build_combat_with_monster(Guardian::new());
        g.run_action(IncreaseMaxHPAction(999));

        assert_matches!(g.monsters[0].behavior.get_intent(), Intent::Defend);
        assert_eq!(
            g.monsters[0].creature.get_status(Status::ModeShift),
            Some(40)
        );

        g.throw_potion(Potion::Fire, Some(CreatureRef::monster(0)));
        assert_eq!(
            g.monsters[0].creature.get_status(Status::ModeShift),
            Some(20)
        );
        assert_matches!(g.monsters[0].behavior.get_intent(), Intent::Defend);

        g.throw_potion(Potion::Fire, Some(CreatureRef::monster(0)));
        assert_eq!(g.monsters[0].creature.get_status(Status::ModeShift), None);
        assert_matches!(g.monsters[0].behavior.get_intent(), Intent::Buff);
        assert_eq!(g.monsters[0].creature.get_status(Status::SharpHide), None);

        g.step_test(EndTurnStep);
        assert_eq!(
            g.monsters[0].creature.get_status(Status::SharpHide),
            Some(4)
        );
        assert_eq!(g.monsters[0].creature.get_status(Status::ModeShift), None);
        g.step_test(EndTurnStep);
        assert_eq!(g.monsters[0].creature.get_status(Status::ModeShift), None);
        assert_eq!(
            g.monsters[0].creature.get_status(Status::SharpHide),
            Some(4)
        );
        g.step_test(EndTurnStep);
        assert_eq!(g.monsters[0].creature.get_status(Status::SharpHide), None);
        assert_eq!(
            g.monsters[0].creature.get_status(Status::ModeShift),
            Some(50)
        );

        g.step_test(EndTurnStep);
        g.throw_potion(Potion::Fire, Some(CreatureRef::monster(0)));
        g.throw_potion(Potion::Fire, Some(CreatureRef::monster(0)));
        g.throw_potion(Potion::Fire, Some(CreatureRef::monster(0)));
        assert_eq!(g.monsters[0].creature.get_status(Status::ModeShift), None);
        assert_matches!(g.monsters[0].behavior.get_intent(), Intent::Buff);
    }

    #[test]
    fn test_mode_shift_from_thorns() {
        let mut g = GameBuilder::default()
            .add_player_status(Status::Thorns, 50)
            .build_combat_with_monster(Guardian::new());
        g.run_action(IncreaseMaxHPAction(999));

        assert_eq!(
            g.monsters[0].creature.get_status(Status::ModeShift),
            Some(40)
        );
        g.step_test(EndTurnStep);
        assert_eq!(
            g.monsters[0].creature.get_status(Status::ModeShift),
            Some(40)
        );
        g.step_test(EndTurnStep);
        assert_eq!(g.monsters[0].creature.get_status(Status::ModeShift), None);
        assert_eq!(g.monsters[0].creature.get_status(Status::SharpHide), None);
        assert_matches!(g.monsters[0].behavior.get_intent(), Intent::Buff);
        g.step_test(EndTurnStep);
        assert_matches!(g.monsters[0].behavior.get_intent(), Intent::Attack(10, 1));
    }
}
