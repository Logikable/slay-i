use std::{cell::RefCell, rc::Rc};

use crate::{
    cards::{CardClass, CardCost, CardType},
    game::CreatureRef,
};

#[derive(Clone)]
pub struct CardPlayInfo<'a> {
    pub card: &'a Card,
    pub target: Option<CreatureRef>,
    pub upgraded: bool,
    pub upgrade_count: i32,
    pub base_increase: i32,
    pub cost: i32,
}

#[derive(Clone)]
pub struct Card {
    pub class: CardClass,
    pub upgrade_count: i32,
    pub cost: CardCost,
    pub exhaust: bool,
    pub is_bottled: bool,
    pub base_increase: i32,
    pub id: u32,
}

impl Card {
    pub fn can_upgrade(&self) -> bool {
        (self.upgrade_count == 0 || self.class.can_upgrade_forever())
            && !matches!(self.class.ty(), CardType::Status | CardType::Curse)
    }
    pub fn upgrade(&mut self) {
        assert!(self.can_upgrade());
        self.upgrade_count += 1;
        if self.class.upgrade_removes_exhaust() {
            self.exhaust = false;
        }
        if let CardCost::Cost { base_cost, .. } = self.cost
            && let Some(new_cost) = self.class.upgrade_cost(base_cost)
        {
            self.update_cost(new_cost);
        }
    }
    pub fn update_cost(&mut self, new_cost: i32) {
        assert!(new_cost >= 0);
        if let CardCost::Cost {
            base_cost,
            temporary_cost,
            ..
        } = &mut self.cost
        {
            if *base_cost != 0 {
                let prev_base_cost = *base_cost;
                *base_cost = new_cost;
                // temporary cost gets adjusted the same amount
                if let Some(temp) = temporary_cost {
                    *temp += new_cost - prev_base_cost;
                    if *temp < 0 {
                        *temp = 0;
                    }
                }
            }
        } else {
            panic!();
        }
    }
    pub fn set_temporary_cost(&mut self, new_cost: i32) {
        assert!(new_cost >= 0);
        if let CardCost::Cost { temporary_cost, .. } = &mut self.cost {
            *temporary_cost = Some(new_cost);
        } else {
            panic!();
        }
    }
    pub fn is_innate(&self) -> bool {
        use CardClass::*;
        match self.class {
            Brutality => self.upgrade_count != 0,
            DramaticEntrance | MindBlast | Writhe => true,
            _ => false,
        }
    }
    pub fn is_ethereal(&self) -> bool {
        use CardClass::*;
        match self.class {
            GhostlyArmor | Carnage | Dazed | Void | AscendersBane | Clumsy => true,
            Apparition => self.upgrade_count == 0,
            _ => false,
        }
    }
    pub fn has_target(&self) -> bool {
        use CardClass::*;
        match self.class {
            Strike | Bash | PommelStrike | TwinStrike | Clothesline | BodySlam | IronWave
            | WildStrike | Headbutt | PerfectedStrike | HeavyBlade | Anger | Clash
            | SearingBlow | Rampage | Uppercut | SeverSoul | Carnage | Hemokinesis | Dropkick
            | Pummel | BloodForBlood | RecklessCharge | SpotWeakness | Disarm | Bludgeon | Feed
            | FiendFire | SwiftStrike | FlashOfSteel | MindBlast | Bite | RitualDagger
            | HandOfGreed | DarkShackles | DebugKill => true,
            Blind | Trip => self.upgrade_count == 0,
            _ => false,
        }
    }
    pub fn clear_temporary(&mut self) {
        match &mut self.cost {
            CardCost::Cost { temporary_cost, .. } => {
                *temporary_cost = None;
            }
            CardCost::X | CardCost::Zero => {}
        }
    }
    pub fn get_base_cost(&self) -> i32 {
        match self.cost {
            CardCost::Cost { base_cost, .. } => base_cost,
            _ => unreachable!(),
        }
    }
    pub fn can_remove_from_master_deck(&self) -> bool {
        !matches!(
            self.class,
            CardClass::AscendersBane | CardClass::CurseOfTheBell
        ) && !self.is_bottled
    }
}

#[cfg(test)]
impl Card {
    pub fn set_cost(&mut self, base: i32, temp: Option<i32>) {
        match &mut self.cost {
            CardCost::Cost {
                base_cost,
                temporary_cost,
                ..
            } => {
                *base_cost = base;
                *temporary_cost = temp;
            }
            _ => unreachable!(),
        }
    }
    pub fn set_free_to_play_once(&mut self) {
        match &mut self.cost {
            CardCost::Cost {
                free_to_play_once, ..
            } => {
                *free_to_play_once = true;
            }
            _ => unreachable!(),
        }
    }
    pub fn get_temporary_cost(&self) -> Option<i32> {
        match self.cost {
            CardCost::Cost { temporary_cost, .. } => temporary_cost,
            _ => unreachable!(),
        }
    }
}

impl std::fmt::Debug for Card {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.class)?;
        for _ in 0..(self.upgrade_count) {
            write!(f, "+")?;
        }
        match self.cost {
            CardCost::Zero | CardCost::X => {}
            CardCost::Cost {
                base_cost,
                temporary_cost,
                free_to_play_once,
            } => {
                write!(f, ", {base_cost} cost")?;
                if let Some(temporary_cost) = temporary_cost {
                    write!(f, " (temp cost {temporary_cost})")?;
                }
                if free_to_play_once {
                    write!(f, " (free to play once)")?;
                }
            }
        }
        Ok(())
    }
}

pub type CardRef = Rc<RefCell<Card>>;
pub type CardPile = Vec<CardRef>;

#[cfg(test)]
mod tests {
    use crate::{
        cards::{CardClass, CardCost},
        game::GameBuilder,
        potion::Potion,
    };

    #[test]
    fn test_upgrade_temp_cost() {
        let mut g = GameBuilder::default().build_combat();
        for (init_temp, final_temp) in [(3, 2), (2, 1), (1, 0), (0, 0)] {
            let c = g.new_card(CardClass::BodySlam);
            let mut c = c.borrow_mut();
            match &mut c.cost {
                CardCost::Cost {
                    base_cost: _,
                    temporary_cost,
                    free_to_play_once: _,
                } => {
                    *temporary_cost = Some(init_temp);
                }
                _ => unreachable!(),
            }
            assert_eq!(
                c.cost,
                CardCost::Cost {
                    base_cost: 1,
                    temporary_cost: Some(init_temp),
                    free_to_play_once: false,
                }
            );
            c.upgrade();
            assert_eq!(
                c.cost,
                CardCost::Cost {
                    base_cost: 0,
                    temporary_cost: Some(final_temp),
                    free_to_play_once: false,
                }
            );
        }
    }

    #[test]
    fn test_upgrade_temp_cost_2() {
        let mut g = GameBuilder::default().build_combat();

        let c = g.new_card(CardClass::DarkEmbrace);
        c.borrow_mut().set_cost(3, None);
        g.hand.push(c);

        let c = g.new_card(CardClass::DarkEmbrace);
        c.borrow_mut().set_cost(0, None);
        g.hand.push(c);

        g.throw_potion(Potion::Forge, None);

        assert_eq!(g.hand[0].borrow().get_base_cost(), 1);
        assert_eq!(g.hand[1].borrow().get_base_cost(), 0);
    }
}
