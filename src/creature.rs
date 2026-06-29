use std::collections::HashMap;

use crate::{
    actions::{
        block::BlockAction, damage::DamageAction, damage_all_monsters::DamageAllMonstersAction,
        draw::DrawAction, gain_energy::GainEnergyAction, gain_status::GainStatusAction,
        heal::HealAction, magnetism::MagnetismAction, mayhem::MayhemAction,
        play_card::PlayCardAction, reduce_status::ReduceStatusAction,
        remove_status::RemoveStatusAction,
    },
    cards::CardType,
    game::CreatureRef,
    queue::ActionQueue,
    status::Status,
};

#[derive(Debug, Default, Clone)]
pub enum CreatureState {
    #[default]
    Alive,
    Dead,
    Escaped,
}

#[derive(Default, Clone)]
pub struct Creature {
    pub name: &'static str,
    pub max_hp: i32,
    pub cur_hp: i32,
    pub block: i32,
    pub last_damage_taken: i32,
    pub state: CreatureState,
    statuses: HashMap<Status, i32>,
}

impl Creature {
    pub fn new(name: &'static str, max_hp: i32) -> Self {
        Self {
            name,
            max_hp,
            cur_hp: max_hp,
            block: 0,
            statuses: Default::default(),
            state: Default::default(),
            last_damage_taken: 0,
        }
    }

    pub fn is_actionable(&self) -> bool {
        matches!(self.state, CreatureState::Alive)
    }

    pub fn is_bloodied(&self) -> bool {
        self.cur_hp <= (self.max_hp + 1) / 2
    }

    pub fn heal(&mut self, amount: i32) {
        self.cur_hp += amount;
        if self.cur_hp > self.max_hp {
            self.cur_hp = self.max_hp;
        }
    }

    pub fn increase_max_hp(&mut self, amount: i32) {
        self.max_hp += amount;
    }

    pub fn decrease_max_hp(&mut self, amount: i32) {
        self.max_hp -= amount;
        self.cur_hp = self.cur_hp.min(self.max_hp);
    }

    pub fn has_any_status(&self) -> bool {
        !self.statuses.is_empty()
    }

    pub fn has_status(&self, status: Status) -> bool {
        self.statuses.contains_key(&status)
    }

    pub fn get_status(&self, status: Status) -> Option<i32> {
        self.statuses.get(&status).copied()
    }

    pub fn remove_status(&mut self, status: Status) {
        self.statuses.remove(&status);
    }

    pub fn clear_all_status(&mut self) {
        self.statuses.clear();
    }

    pub fn set_status(&mut self, status: Status, amount: i32) {
        self.statuses.insert(status, amount);
    }

    pub fn all_statuses(&self) -> std::collections::hash_map::Iter<'_, Status, i32> {
        self.statuses.iter()
    }

    pub fn start_of_turn_lose_block(&mut self, has_calipers: bool) {
        if self.has_status(Status::Barricade) {
            return;
        }
        if has_calipers {
            self.block = 0.max(self.block - 15);
        } else {
            self.block = 0;
        }
    }

    pub fn trigger_statuses_on_card_played(
        &mut self,
        queue: &mut ActionQueue,
        card_queue: &mut Vec<PlayCardAction>,
        play: &PlayCardAction,
    ) {
        for (p, p_next) in [
            (Status::Panache5, Status::Panache4),
            (Status::Panache4, Status::Panache3),
            (Status::Panache3, Status::Panache2),
            (Status::Panache2, Status::Panache1),
            (Status::Panache1, Status::Panache5),
        ] {
            if let Some(v) = self.get_status(p) {
                self.set_status(p_next, v);
                self.remove_status(p);
                if p == Status::Panache1 {
                    queue.push_bot(DamageAllMonstersAction::thorns(v));
                }
                break;
            }
        }
        if !play.is_duplicated {
            if self.has_status(Status::Duplication) {
                queue.push_bot(ReduceStatusAction {
                    status: Status::Duplication,
                    amount: 1,
                    target: CreatureRef::player(),
                });
                card_queue.push(PlayCardAction::duplicated(play));
            }
            if self.has_status(Status::DoubleTap)
                && play.card.borrow().class.ty() == CardType::Attack
            {
                queue.push_bot(ReduceStatusAction {
                    status: Status::DoubleTap,
                    amount: 1,
                    target: CreatureRef::player(),
                });
                card_queue.push(PlayCardAction::duplicated(play));
            }
            if play.card.borrow().class.ty() == CardType::Attack {
                if self.has_status(Status::PenNib) {
                    queue.push_bot(RemoveStatusAction {
                        status: Status::PenNib,
                        target: CreatureRef::player(),
                    });
                }
                if self.has_status(Status::Vigor) {
                    queue.push_bot(RemoveStatusAction {
                        status: Status::Vigor,
                        target: CreatureRef::player(),
                    });
                }
            }
            if let Some(v) = self.get_status(Status::Rage)
                && play.card.borrow().class.ty() == CardType::Attack
            {
                queue.push_bot(BlockAction::player_flat_amount(v));
            }
        }
    }

    pub fn trigger_statuses_turn_begin(&mut self, this: CreatureRef, queue: &mut ActionQueue) {
        if let Some(v) = self.get_status(Status::Magnetism) {
            for _ in 0..v {
                queue.push_bot(MagnetismAction());
            }
        }
        if let Some(v) = self.get_status(Status::Mayhem) {
            for _ in 0..v {
                queue.push_bot(MayhemAction());
            }
        }
        if let Some(v) = self.get_status(Status::Berserk) {
            queue.push_bot(GainEnergyAction(v));
        }
        if let Some(v) = self.get_status(Status::NextTurnBlock) {
            queue.push_bot(BlockAction::player_flat_amount(v));
            queue.push_bot(RemoveStatusAction {
                status: Status::NextTurnBlock,
                target: this,
            });
        }
        for status in [Status::NextTurnBlock, Status::FlameBarrier] {
            if self.has_status(status) {
                queue.push_bot(RemoveStatusAction {
                    status,
                    target: this,
                });
            }
        }
    }

    pub fn trigger_statuses_turn_begin_post_draw(
        &mut self,
        this: CreatureRef,
        queue: &mut ActionQueue,
    ) {
        if let Some(v) = self.get_status(Status::DemonForm) {
            queue.push_bot(GainStatusAction {
                status: Status::Strength,
                amount: v,
                target: this,
            });
        }
        if let Some(v) = self.get_status(Status::Brutality) {
            queue.push_bot(DrawAction(v));
            queue.push_bot(DamageAction::lose_hp(v, this));
        }
    }

    pub fn trigger_statuses_turn_end(&mut self, this: CreatureRef, queue: &mut ActionQueue) {
        if let Some(v) = self.get_status(Status::LoseDexterity) {
            queue.push_bot(GainStatusAction {
                status: Status::Dexterity,
                amount: -v,
                target: this,
            });
            queue.push_bot(RemoveStatusAction {
                status: Status::LoseDexterity,
                target: this,
            });
        }
        if let Some(v) = self.get_status(Status::LoseStrength) {
            queue.push_bot(GainStatusAction {
                status: Status::Strength,
                amount: -v,
                target: this,
            });
            queue.push_bot(RemoveStatusAction {
                status: Status::LoseStrength,
                target: this,
            });
        }
        if let Some(v) = self.get_status(Status::Ritual) {
            queue.push_bot(GainStatusAction {
                status: Status::Strength,
                amount: v,
                target: this,
            });
        }
        if let Some(v) = self.get_status(Status::GainStrength) {
            queue.push_bot(GainStatusAction {
                status: Status::Strength,
                amount: v,
                target: this,
            });
            queue.push_bot(RemoveStatusAction {
                status: Status::GainStrength,
                target: this,
            });
        }
        if let Some(v) = self.get_status(Status::Metallicize) {
            queue.push_bot(BlockAction::monster(this, v));
        }
        if let Some(v) = self.get_status(Status::PlatedArmor) {
            queue.push_bot(BlockAction::monster(this, v));
        }
        if let Some(v) = self.get_status(Status::CombustHPLoss) {
            queue.push_bot(DamageAction::lose_hp(v, this));
        }
        if let Some(v) = self.get_status(Status::CombustDamage) {
            queue.push_bot(DamageAllMonstersAction::thorns(v));
        }
        if let Some(b) = self.get_status(Status::Bomb1) {
            queue.push_bot(DamageAllMonstersAction::thorns(b));
            self.remove_status(Status::Bomb1);
        }
        if let Some(b) = self.get_status(Status::Bomb2) {
            self.set_status(Status::Bomb1, b);
            self.remove_status(Status::Bomb2);
        }
        if let Some(b) = self.get_status(Status::Bomb3) {
            self.set_status(Status::Bomb2, b);
            self.remove_status(Status::Bomb3);
        }
        if let Some(v) = self.get_status(Status::RegenPlayer) {
            // yes, this is push_top
            queue.push_top(HealAction {
                target: this,
                amount: v,
            });
            queue.push_top(ReduceStatusAction {
                status: Status::RegenPlayer,
                amount: 1,
                target: this,
            });
        }
        if let Some(v) = self.get_status(Status::RegenMonster) {
            queue.push_bot(HealAction {
                target: this,
                amount: v,
            });
        }
        for p in [
            Status::Panache4,
            Status::Panache3,
            Status::Panache2,
            Status::Panache1,
        ] {
            if let Some(v) = self.get_status(p) {
                self.set_status(Status::Panache5, v);
                self.remove_status(p);
                break;
            }
        }
        if self.has_status(Status::Rage) {
            queue.push_bot(RemoveStatusAction {
                status: Status::Rage,
                target: this,
            });
        }
    }
    pub fn trigger_statuses_round_end(&mut self, this: CreatureRef, queue: &mut ActionQueue) {
        for s in self.statuses.keys() {
            if s.decays() {
                queue.push_bot(ReduceStatusAction {
                    status: *s,
                    amount: 1,
                    target: this,
                });
            }
            if s.disappears_end_of_turn() {
                queue.push_bot(RemoveStatusAction {
                    status: *s,
                    target: this,
                });
            }
        }
    }

    pub fn str(&self) -> String {
        let mut s = format!(
            "{}: {}/{}, {} block",
            self.name, self.cur_hp, self.max_hp, self.block
        );
        if self.has_any_status() {
            let mut first = true;
            for (status, amount) in self.all_statuses() {
                if first {
                    first = false;
                    s += ", statuses: ";
                } else {
                    s += ", ";
                }
                s.push_str(&format!("{status:?} ({amount})"));
            }
        }
        s
    }
}
