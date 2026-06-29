//! Random-play combat fuzzer. Drives full Act-1 combats with random legal
//! Ironclad decks, picking random valid steps until the run ends, checking
//! invariants and catching panics. Not part of the normal suite; run with:
//!   cargo test --release fuzz_combat -- --ignored --nocapture
//! Iteration count is configurable via the FUZZ_ITERS env var.

#[cfg(test)]
mod tests {
    use std::panic::{AssertUnwindSafe, catch_unwind};

    use rand::RngExt;

    use crate::{
        cards::{CardClass, CardColor, CardType},
        game::{GameBuilder, GameStatus, Rand},
        map::RoomType,
    };

    fn fuzzable_cards() -> Vec<CardClass> {
        CardClass::all()
            .into_iter()
            .filter(|c| matches!(c.color(), CardColor::Red | CardColor::Colorless))
            .filter(|c| !matches!(c.ty(), CardType::Curse | CardType::Status))
            .collect()
    }

    fn check_invariants(game: &crate::game::Game) {
        assert!(game.energy >= 0, "negative energy: {}", game.energy);
        let p = &game.player;
        assert!(
            p.cur_hp <= p.max_hp,
            "player cur_hp {} > max_hp {}",
            p.cur_hp,
            p.max_hp
        );
        for m in &game.monsters {
            let c = &m.creature;
            assert!(
                c.cur_hp <= c.max_hp,
                "monster cur_hp {} > max_hp {}",
                c.cur_hp,
                c.max_hp
            );
        }
    }

    // True if `desc` is an "ascend to (x, y)" step with y past the last room.
    fn ascends_past_path(desc: &str, nrooms: usize) -> bool {
        let Some(rest) = desc.strip_prefix("ascend to (") else {
            return false;
        };
        let Some((_, y)) = rest.trim_end_matches(')').split_once(", ") else {
            return false;
        };
        y.parse::<usize>().map(|y| y >= nrooms).unwrap_or(false)
    }

    // Plays one random run to completion, appending each chosen step to `log`.
    // Returns true if the run stalled (hit the step cap without terminating).
    // Stalls are legitimate non-terminating game states reachable under random
    // play (e.g. a weak deck debuffed to 0 damage vs. Lagavulin's Metallicize),
    // not engine bugs, so they are counted separately rather than failing.
    fn run_one(rng: &mut Rand, pool: &[CardClass], log: &mut Vec<String>) -> bool {
        let mut builder = GameBuilder::default().ironclad_starting_deck();
        let extra = rng.random_range(0..16);
        let mut deck = vec![];
        for _ in 0..extra {
            let c = pool[rng.random_range(0..pool.len())];
            deck.push(c);
            builder = builder.add_card(c);
        }
        log.push(format!("deck +{extra}: {deck:?}"));

        const NROOMS: usize = 8;
        let rooms: Vec<RoomType> = (0..NROOMS)
            .map(|_| {
                if rng.random_range(0..4) == 0 {
                    RoomType::Elite
                } else {
                    RoomType::Monster
                }
            })
            .collect();
        let mut game = builder.build_with_rooms(&rooms);

        let mut steps_taken = 0;
        loop {
            match game.status {
                GameStatus::Victory | GameStatus::Defeat => break,
                GameStatus::Combat => {}
            }
            let steps = game.valid_steps();
            // The straight test path dangles an edge off the last room; never
            // ascend past it (that node is unset). Treat it as the run's end.
            let candidates: Vec<usize> = (0..steps.len())
                .filter(|&i| !ascends_past_path(&steps[i].description(&game), NROOMS))
                .collect();
            if candidates.is_empty() {
                break;
            }
            check_invariants(&game);
            let idx = candidates[rng.random_range(0..candidates.len())];
            log.push(format!("{idx}: {}", steps[idx].description(&game)));
            game.step(idx);
            steps_taken += 1;
            if steps_taken > 10000 {
                return true;
            }
        }
        false
    }

    #[test]
    #[ignore]
    fn fuzz_combat() {
        let iters: usize = std::env::var("FUZZ_ITERS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(2000);
        let pool = fuzzable_cards();
        let mut rng = Rand::default();

        let mut failures = vec![];
        let mut stalls = 0;
        for iter in 0..iters {
            let mut log = vec![];
            let res = catch_unwind(AssertUnwindSafe(|| run_one(&mut rng, &pool, &mut log)));
            match res {
                Ok(true) => stalls += 1,
                Ok(false) => {}
                Err(e) => {
                    let msg = e
                        .downcast_ref::<&str>()
                        .map(|s| s.to_string())
                        .or_else(|| e.downcast_ref::<String>().cloned())
                        .unwrap_or_else(|| "<non-string panic>".to_string());
                    failures.push((iter, msg, log));
                    if failures.len() >= 15 {
                        break;
                    }
                }
            }
        }

        if !failures.is_empty() {
            for (iter, msg, log) in &failures {
                println!("\n=== FAIL iter {iter}: {msg} ===");
                for l in log {
                    println!("  {l}");
                }
            }
            panic!("{} fuzz failure(s)", failures.len());
        }
        println!("fuzz_combat: {iters} runs OK ({stalls} stalled)");
    }
}
