use rand::RngExt;

use crate::creature::Creature;
use crate::game::{CreatureRef, Game, Rand};
use crate::queue::ActionQueue;

#[derive(Debug, Clone, Copy)]
pub enum Intent {
    Attack(i32, i32),
    AttackBuff(i32, i32),
    AttackDebuff(i32, i32),
    AttackDefend(i32, i32),
    Buff,
    Debuff,
    StrongDebuff,
    Defend,
    DefendBuff,
    DefendDebuff,
    Escape,
    Sleep,
    Stun,
    Unknown,
}

impl Intent {
    pub fn is_attack(&self) -> bool {
        use Intent::*;
        matches!(
            self,
            Attack(..) | AttackBuff(..) | AttackDebuff(..) | AttackDefend(..)
        )
    }
    pub fn modify_damage(&mut self, this: CreatureRef, game: &Game) {
        use Intent::*;
        let d = match self {
            Attack(d, _) => d,
            AttackBuff(d, _) => d,
            AttackDebuff(d, _) => d,
            AttackDefend(d, _) => d,
            _ => {
                return;
            }
        };
        *d = game.calculate_damage(*d, this, CreatureRef::player());
    }
}

#[derive(Debug, Clone)]
pub struct MonsterInfo {
    pub num_alive_monsters: usize,
    pub player_hp: i32,
}

pub trait MonsterBehavior {
    fn name(&self) -> &'static str;
    fn hp_range(&self) -> (i32, i32);
    fn pre_combat(&self, _queue: &mut ActionQueue, _this: CreatureRef, _rng: &mut Rand) {}
    fn on_take_damage(&mut self, _this: CreatureRef, _this_creature: &mut Creature) {}
    fn roll_next_action(&mut self, r: &mut Rand, info: &MonsterInfo);
    fn take_turn(&mut self, this: CreatureRef, queue: &mut ActionQueue, info: &MonsterInfo);
    fn get_intent(&self) -> Intent;
    // Forks this behavior for `Game::clone_for_search`. Monsters used as search
    // targets implement this; others panic so unsupported scenarios fail loudly.
    fn clone_box(&self) -> Box<dyn MonsterBehavior> {
        panic!("clone_box not implemented for monster {}", self.name())
    }
}

pub struct Monster {
    pub creature: Creature,
    pub behavior: Box<dyn MonsterBehavior>,
}

impl Monster {
    pub fn new<M: MonsterBehavior + 'static>(m: M, rng: &mut Rand) -> Self {
        Self::new_boxed(Box::new(m), rng)
    }
    pub fn new_boxed(m: Box<dyn MonsterBehavior>, rng: &mut Rand) -> Self {
        let (lo, hi) = m.hp_range();
        let hp = rng.random_range(lo..=hi);
        let name = m.name();

        Monster {
            creature: Creature::new(name, hp),
            behavior: m,
        }
    }
    pub fn new_with_hp<M: MonsterBehavior + 'static>(m: M, hp: i32) -> Self {
        let name = m.name();
        Monster {
            creature: Creature::new(name, hp),
            behavior: Box::new(m),
        }
    }
    pub fn clone_for_search(&self) -> Monster {
        Monster {
            creature: self.creature.clone(),
            behavior: self.behavior.clone_box(),
        }
    }
}
