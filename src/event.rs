use rand::RngExt;

use crate::{
    actions::gain_gold::GainGoldAction,
    combat::RollCombatGameState,
    game::{Game, RollShopGameState, RollTreasureGameState, RunActionsGameState},
    map::RoomType,
    relic::RelicClass,
    rng::remove_random,
    state::GameState,
};

#[derive(Debug, PartialEq, Eq)]
enum EventType {
    Monster,
    Shop,
    Chest,
    Event,
}

#[derive(Debug)]
pub struct RollEventGameState;

impl GameState for RollEventGameState {
    fn run(&self, game: &mut Game) {
        game.cur_room = Some(RoomType::Event);
        // for testing only
        let e = if !game.override_event_queue.is_empty() {
            game.override_event_queue.remove(0)
        } else {
            // 25% for shrine or one time event
            // 75% for act-specific event
            let mut events;
            if game.rng.random_range(0..4) == 0 {
                events = game.event_shrine_pool.clone();
                for e in &game.event_one_time_pool {
                    events.push(*e);
                }
            } else {
                events = game.event_act_pool.clone();
            }
            events.retain(|e| e.can_spawn(game));
            // A long climb can exhaust the chosen pool (each seen event is
            // removed from every pool below), leaving nothing to sample. Fall
            // back to whatever spawnable events remain across all pools so the
            // room still resolves instead of panicking on an empty range.
            if events.is_empty() {
                events = game.event_act_pool.clone();
                events.extend(game.event_shrine_pool.iter().copied());
                events.extend(game.event_one_time_pool.iter().copied());
                events.retain(|e| e.can_spawn(game));
            }
            // No spawnable event anywhere (pathological): resolve as an empty
            // room rather than crash.
            if events.is_empty() {
                return;
            }
            let e = remove_random(&mut game.rng, &mut events);
            game.event_act_pool.retain(|&e2| e2 != e);
            game.event_one_time_pool.retain(|&e2| e2 != e);
            game.event_shrine_pool.retain(|&e2| e2 != e);
            e
        };
        game.cur_event = Some(e);
        let e = e.game_state(game);
        game.state.push_boxed_state(e);
    }
}

#[derive(Debug)]
pub struct RollQuestionRoomGameState;

impl GameState for RollQuestionRoomGameState {
    fn run(&self, game: &mut Game) {
        let roll = game.rng.random_range(0..100);
        let event_shop_chance = if game.cur_room == Some(RoomType::Shop) {
            0
        } else {
            game.event_shop_chance
        };
        let mut ty = if roll < game.event_monster_chance {
            EventType::Monster
        } else if roll < game.event_monster_chance + event_shop_chance {
            EventType::Shop
        } else if roll < game.event_monster_chance + event_shop_chance + game.event_chest_chance {
            EventType::Chest
        } else {
            EventType::Event
        };
        // for testing only
        if !game.override_event_queue.is_empty() {
            ty = EventType::Event;
        }
        if let Some(mut v) = game.get_relic_value(RelicClass::TinyChest) {
            v += 1;
            if v == 4 {
                v = 0;
                ty = EventType::Chest;
            }
            game.set_relic_value(RelicClass::TinyChest, v);
        }
        if ty == EventType::Monster {
            game.event_monster_chance = 10;
            if game.has_relic(RelicClass::JuzuBracelet) {
                ty = EventType::Event;
            }
        } else {
            game.event_monster_chance += 10;
        }
        if ty == EventType::Shop {
            game.event_shop_chance = 3;
        } else {
            game.event_shop_chance += 3;
        }
        if ty == EventType::Chest {
            game.event_shop_chance = 2;
        } else {
            game.event_shop_chance += 2;
        }
        match ty {
            EventType::Monster => game.state.push_state(RollCombatGameState),
            EventType::Shop => game.state.push_state(RollShopGameState),
            EventType::Chest => game.state.push_state(RollTreasureGameState),
            EventType::Event => game.state.push_state(RollEventGameState),
        }
        if game.has_relic(RelicClass::SsserpentHead) {
            game.action_queue.push_bot(GainGoldAction(50));
            game.state.push_state(RunActionsGameState);
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        chest::OpenChestStep,
        events::Event,
        game::{AscendStep, GameBuilder},
        map::RoomType,
        relic::RelicClass,
        shop::ShopExitStep,
        state::ContinueStep,
        step::Step,
    };

    #[test]
    fn test_event_shop() {
        for _ in 0..20 {
            let mut g = GameBuilder::default().build_with_rooms(&[RoomType::Shop, RoomType::Event]);
            g.step_test(AscendStep::new(0, 0));
            g.step_test(ShopExitStep);
            g.step_test(AscendStep::new(0, 1));
            // cannot have event shop after shop
            assert!(
                !g.valid_steps()
                    .contains(&(Box::new(ShopExitStep) as Box<dyn Step>))
            )
        }
    }

    #[test]
    fn test_tiny_chest() {
        let mut g = GameBuilder::default()
            .add_relic(RelicClass::TinyChest)
            .build_with_rooms(&[
                RoomType::Event,
                RoomType::Event,
                RoomType::Event,
                RoomType::Event,
            ]);
        for _ in 0..4 {
            g.override_event_queue.push(Event::Transmorgrifier);
        }
        assert_eq!(g.get_relic_value(RelicClass::TinyChest), Some(0));
        g.step_test(AscendStep::new(0, 0));
        assert_eq!(g.get_relic_value(RelicClass::TinyChest), Some(1));
        g.step_test(ContinueStep);
        g.step_test(AscendStep::new(0, 1));
        assert_eq!(g.get_relic_value(RelicClass::TinyChest), Some(2));
        g.step_test(ContinueStep);
        g.step_test(AscendStep::new(0, 2));
        assert_eq!(g.get_relic_value(RelicClass::TinyChest), Some(3));
        g.step_test(ContinueStep);
        g.step_test(AscendStep::new(0, 3));
        assert_eq!(g.get_relic_value(RelicClass::TinyChest), Some(0));
        g.step_test(OpenChestStep);
    }
}
