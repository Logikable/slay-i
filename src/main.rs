mod action;
mod actions;
mod blessings;
mod campfire;
mod card;
mod cards;
mod chest;
mod combat;
mod creature;
mod draw_pile;
mod event;
mod events;
#[cfg(test)]
mod fuzz;
mod game;
mod map;
mod master_deck;
mod monster;
mod monsters;
mod move_history;
#[cfg(test)]
mod player;
mod potion;
mod queue;
mod relic;
mod rewards;
mod rng;
mod shop;
mod state;
mod status;
mod step;
mod test;

use game::Game;

// Thread-local heaps: the ISMCTS search forks combats by deep-cloning the card
// Rc graph, so the player + parallel eval allocate heavily across threads.
// glibc malloc serializes that (multi-thread sweeps collapsed to ~1-2 cores);
// mimalloc keeps them near-linear.
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

use crate::{
    cards::CardClass,
    game::{GameBuilder, GameStatus},
    relic::RelicClass,
};

fn print_state(g: &Game) {
    println!("{}", g.player.str());
    println!("relics:");
    for r in &g.relics {
        println!(" {:?}: {}", r.get_class(), r.get_value());
    }
    if g.potions.iter().any(|p| p.is_some()) {
        print!("potions:");
        for p in g.potions.iter().flatten() {
            print!(" {p:?}");
        }
        println!();
    }
    println!("energy: {}", g.energy);
    println!("monsters:");
    for m in g.get_actionable_monsters_in_order() {
        println!(" {}", g.monster_str(m));
    }
    println!("hand:");
    for c in &g.hand {
        println!(" {:?}", c.borrow());
    }
    println!("draw pile:");
    for c in g.draw_pile.get_all() {
        println!(" {:?}", c.borrow());
    }
    println!("discard pile:");
    for c in &g.discard_pile {
        println!(" {:?}", c.borrow());
    }
    println!("exhaust pile:");
    for c in &g.exhaust_pile {
        println!(" {:?}", c.borrow());
    }
    if let Some(c) = &g.cur_card {
        println!("current card being played: {:?}", c.borrow());
    }
    if !g.chosen_cards.is_empty() {
        println!("cards being processed:");
        for c in &g.chosen_cards {
            println!(" {:?}", c.borrow());
        }
    }
    println!("moves:");
    for (si, s) in g.valid_steps().iter().enumerate() {
        println!(" {si}: {}", s.description(g));
    }
}

enum UserInput {
    Step(usize),
    PrintMap,
}

fn read_user_input(max: usize) -> UserInput {
    let mut s = String::new();
    loop {
        s.clear();
        std::io::stdin().read_line(&mut s).unwrap();
        s = s.trim().to_owned();
        if s == "m" {
            return UserInput::PrintMap;
        }
        if let Ok(v) = s.parse()
            && v < max
        {
            return UserInput::Step(v);
        }
        if !s.trim().is_empty() {
            println!("invalid num \"{}\"", s.trim());
            println!("number to choose action, \"m\" to print map");
        }
    }
}

fn main() {
    let mut game = GameBuilder::default()
        .ironclad_starting_deck()
        .add_card(CardClass::Armaments)
        .add_card(CardClass::Purity)
        .add_card_upgraded(CardClass::Inflame)
        .add_relic(RelicClass::BurningBlood)
        .build();
    loop {
        match game.status {
            GameStatus::Defeat => {
                println!("defeat :(");
                break;
            }
            GameStatus::Victory => {
                println!("victory! :)");
                break;
            }
            GameStatus::Combat => {
                print_state(&game);
                let valid_steps = game.valid_steps();
                let i = read_user_input(valid_steps.len());
                match i {
                    UserInput::Step(s) => game.step(s),
                    UserInput::PrintMap => game.map.print(),
                }
                println!("-----------------------------");
            }
        }
    }
}
