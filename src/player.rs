//! Fork-only combat player: agents + an evaluation harness. Not for upstream.
//! Run with: cargo test --release player_eval -- --ignored --nocapture

#![cfg(test)]

use rand::RngExt;

use crate::{
    cards::{CardClass, CardCost, CardType},
    combat::{EndTurnStep, PlayCardStep},
    game::{CombatType, CreatureRef, Game, GameStatus, Rand},
    monster::Intent,
    status::Status,
    step::Step,
};

pub trait CombatAgent {
    // Pick an index into game.valid_steps() for the current decision.
    fn act(&mut self, game: &Game) -> usize;
}

fn index_of(steps: &[Box<dyn Step>], want: Box<dyn Step>) -> Option<usize> {
    steps.iter().position(|s| s == &want)
}

// ---------------------------------------------------------------------------
// Random baseline
// ---------------------------------------------------------------------------

pub struct RandomAgent {
    rng: Rand,
}

impl RandomAgent {
    pub fn new() -> Self {
        Self {
            rng: Rand::default(),
        }
    }
}

impl CombatAgent for RandomAgent {
    fn act(&mut self, game: &Game) -> usize {
        let n = game.valid_steps().len();
        self.rng.random_range(0..n)
    }
}

// ---------------------------------------------------------------------------
// Greedy 1-ply heuristic (tuned for the Ironclad basic deck)
// ---------------------------------------------------------------------------

pub struct GreedyAgent;

fn card_cost(class: CardClass) -> i32 {
    match class.base_cost() {
        CardCost::Cost { base_cost, .. } => base_cost,
        CardCost::Zero => 0,
        CardCost::X => 3,
    }
}

// Rough base (pre-mitigation) damage for attacks; None for non-attacks.
fn card_base_damage(class: CardClass) -> Option<i32> {
    use CardClass::*;
    if class.ty() != CardType::Attack {
        return None;
    }
    Some(match class {
        Strike => 6,
        Bash => 8,
        Clothesline => 12,
        Cleave => 8,
        Thunderclap => 4,
        Anger => 6,
        PommelStrike => 9,
        TwinStrike => 10, // 5 x2
        IronWave => 5,
        Headbutt => 9,
        Clash => 14,
        Uppercut => 13,
        Carnage => 20,
        Hemokinesis => 15,
        _ => 6,
    })
}

fn is_block_card(class: CardClass) -> bool {
    matches!(
        class,
        CardClass::Defend | CardClass::ShrugItOff | CardClass::IronWave | CardClass::TrueGrit
    )
}

impl GreedyAgent {
    // Estimated incoming attack damage this turn, after current block.
    fn incoming(&self, game: &Game) -> i32 {
        let mut total = 0;
        for (i, m) in game.monsters.iter().enumerate() {
            if !m.creature.is_actionable() {
                continue;
            }
            let (d, hits) = match m.behavior.get_intent() {
                Intent::Attack(d, h)
                | Intent::AttackBuff(d, h)
                | Intent::AttackDebuff(d, h)
                | Intent::AttackDefend(d, h) => (d, h),
                _ => continue,
            };
            let real = game.calculate_damage(d, CreatureRef::monster(i), CreatureRef::player());
            total += real * hits;
        }
        (total - game.player.block).max(0)
    }

    // Approx burst damage we can deal to `target` this turn given energy.
    fn attack_potential(&self, game: &Game, target: usize) -> i32 {
        let mut attacks: Vec<(i32, i32)> = game
            .hand
            .iter()
            .filter_map(|c| {
                let class = c.borrow().class;
                card_base_damage(class).map(|base| {
                    (
                        card_cost(class),
                        game.calculate_damage(
                            base,
                            CreatureRef::player(),
                            CreatureRef::monster(target),
                        ),
                    )
                })
            })
            .collect();
        attacks.sort_by_key(|&(_, dmg)| -dmg);
        let mut energy = game.energy;
        let mut total = 0;
        for (cost, dmg) in attacks {
            if cost <= energy {
                energy -= cost;
                total += dmg;
            }
        }
        total
    }

    fn decide(&self, game: &Game) -> Box<dyn Step> {
        let steps = game.valid_steps();
        let alive: Vec<usize> = (0..game.monsters.len())
            .filter(|&i| game.monsters[i].creature.is_actionable())
            .collect();
        if alive.is_empty() {
            return Box::new(EndTurnStep);
        }
        // Primary target: lowest effective HP, tie-break by attacker.
        let target = *alive
            .iter()
            .min_by_key(|&&i| {
                let c = &game.monsters[i].creature;
                (
                    c.cur_hp + c.block,
                    !game.monsters[i].behavior.get_intent().is_attack() as i32,
                )
            })
            .unwrap();

        let avail = |ci: usize, t: Option<usize>| {
            index_of(
                &steps,
                Box::new(PlayCardStep {
                    hand_index: ci,
                    target: t,
                }),
            )
            .is_some()
        };

        // Classify playable hand cards (toward the chosen target).
        let mut attacks = vec![]; // (hand_index, class, target_opt)
        let mut blocks = vec![];
        let mut others = vec![];
        for (ci, c) in game.hand.iter().enumerate() {
            let (class, targeted) = {
                let b = c.borrow();
                (b.class, b.has_target())
            };
            let t = if targeted { Some(target) } else { None };
            if targeted && !avail(ci, t) {
                // targetable but not on our target (e.g. unaffordable) -> any target
                let any = alive.iter().find(|&&mi| avail(ci, Some(mi)));
                match any {
                    Some(&mi) => {
                        push_card(&mut attacks, &mut blocks, &mut others, ci, class, Some(mi))
                    }
                    None => continue,
                }
            } else if !targeted && !avail(ci, None) {
                continue;
            } else {
                push_card(&mut attacks, &mut blocks, &mut others, ci, class, t);
            }
        }

        let tc = &game.monsters[target].creature;
        let lethal = self.attack_potential(game, target) >= tc.cur_hp + tc.block;
        // Vs Enrage (Gremlin Nob), skills/powers buff the monster: never play
        // non-attacks, just race it down.
        let enrage = alive
            .iter()
            .any(|&i| game.monsters[i].creature.has_status(Status::Enrage));
        let need_block = if enrage { 0 } else { self.incoming(game) };

        let play = |ci: usize, t: Option<usize>| -> Box<dyn Step> {
            Box::new(PlayCardStep {
                hand_index: ci,
                target: t,
            })
        };

        // 1) If we can kill the target this turn, go all-in (Bash first).
        if lethal {
            if let Some(&(ci, _, t)) = attacks.iter().find(|(_, cl, _)| *cl == CardClass::Bash) {
                return play(ci, t);
            }
            if let Some(&(ci, _, t)) = attacks.first() {
                return play(ci, t);
            }
        }
        // 2) Play Strength/Powers early (they compound); skip vs Enrage and when
        // this turn's hit could kill us (block first then).
        if !enrage && need_block < game.player.cur_hp {
            if let Some(&(ci, _, t)) = others.iter().find(|(_, cl, _)| cl.ty() == CardType::Power) {
                return play(ci, t);
            }
        }
        // Don't wake a sleeping enemy (e.g. Lagavulin) unless we can kill it now;
        // we've already stacked Strength above, so just pass and keep setting up.
        let target_asleep = matches!(game.monsters[target].behavior.get_intent(), Intent::Sleep);
        if target_asleep && !lethal {
            return Box::new(EndTurnStep);
        }
        // 3) Cover incoming damage.
        if need_block > 0 {
            if let Some(&(ci, _, t)) = blocks.first() {
                return play(ci, t);
            }
        }
        // 3) Set up Vulnerable with Bash if the target will survive.
        if !game.monsters[target]
            .creature
            .has_status(Status::Vulnerable)
        {
            if let Some(&(ci, _, t)) = attacks.iter().find(|(_, cl, _)| *cl == CardClass::Bash) {
                return play(ci, t);
            }
        }
        // 4) Otherwise hit, then dump buffs/other (never buffs into Enrage).
        if let Some(&(ci, _, t)) = attacks.first() {
            return play(ci, t);
        }
        if !enrage {
            if let Some(&(ci, _, t)) = others.first() {
                return play(ci, t);
            }
        }
        Box::new(EndTurnStep)
    }
}

#[allow(clippy::type_complexity)]
fn push_card(
    attacks: &mut Vec<(usize, CardClass, Option<usize>)>,
    blocks: &mut Vec<(usize, CardClass, Option<usize>)>,
    others: &mut Vec<(usize, CardClass, Option<usize>)>,
    ci: usize,
    class: CardClass,
    t: Option<usize>,
) {
    if class.ty() == CardType::Attack {
        attacks.push((ci, class, t));
    } else if is_block_card(class) {
        blocks.push((ci, class, t));
    } else {
        others.push((ci, class, t));
    }
}

impl CombatAgent for GreedyAgent {
    fn act(&mut self, game: &Game) -> usize {
        let steps = game.valid_steps();
        let want = self.decide(game);
        index_of(&steps, want).unwrap_or(0)
    }
}

// ---------------------------------------------------------------------------
// Rollout-improvement search: one ply of lookahead over the Greedy policy.
//
// At each decision we fork the game (clone_for_search), apply each candidate
// step, then play the rest of combat with GreedyAgent as the default policy,
// averaged over several reseeded rollouts to sample monster rolls and draws.
// We pick the step with the best average terminal value. This is policy
// improvement over Greedy, so it can only help, and it captures multi-turn
// timing (when to burst, block, or stack) that 1-ply Greedy misses.
// ---------------------------------------------------------------------------

pub struct RolloutAgent {
    rollouts: u32,
    rng: Rand,
}

impl RolloutAgent {
    pub fn new(rollouts: u32) -> Self {
        Self {
            rollouts,
            rng: Rand::default(),
        }
    }
}

// Terminal value of a finished (or capped) rollout. Winning dominates; among
// wins prefer more remaining HP; among losses, surviving longer is less bad.
fn rollout_value(game: &mut Game, outcome: Outcome) -> f64 {
    match outcome {
        Outcome::Win { .. } => 1000.0 + game.player.cur_hp as f64,
        Outcome::Loss => -1000.0 + game.turn as f64,
        Outcome::Stall => {
            let monster_hp: i32 = game
                .monsters
                .iter()
                .filter(|m| m.creature.is_actionable())
                .map(|m| m.creature.cur_hp + m.creature.block)
                .sum();
            game.player.cur_hp as f64 - monster_hp as f64
        }
    }
}

impl CombatAgent for RolloutAgent {
    fn act(&mut self, game: &Game) -> usize {
        let steps = game.valid_steps();
        if steps.len() <= 1 {
            return 0;
        }
        let mut best = 0;
        let mut best_score = f64::MIN;
        for i in 0..steps.len() {
            let mut total = 0.0;
            for _ in 0..self.rollouts {
                let mut sim = game.clone_for_search();
                // Reseed so each rollout samples a different future.
                let seed: u64 = self.rng.random();
                sim.rng = Rand::seed_from_u64(seed);
                sim.step(i);
                let mut policy = GreedyAgent;
                let outcome = play_out(&mut sim, &mut policy);
                total += rollout_value(&mut sim, outcome);
            }
            let avg = total / self.rollouts as f64;
            if avg > best_score {
                best_score = avg;
                best = i;
            }
        }
        best
    }
}

// ---------------------------------------------------------------------------
// Evaluation harness
// ---------------------------------------------------------------------------

#[derive(Default)]
struct Stats {
    wins: u32,
    trials: u32,
    hp_lost_on_win: u64,
    turns_on_win: u64,
    stalls: u32,
}

enum Outcome {
    Win { hp_lost: i32, turns: i32 },
    Loss,
    Stall,
}

fn play_out<A: CombatAgent>(game: &mut Game, agent: &mut A) -> Outcome {
    let start_hp = game.player.cur_hp;
    let end_turn: Box<dyn Step> = Box::new(EndTurnStep);
    let mut turns = 0;
    let mut steps_taken = 0;
    loop {
        if matches!(game.status, GameStatus::Defeat) {
            return Outcome::Loss;
        }
        if game.in_combat == CombatType::None {
            break; // combat resolved without defeat -> win
        }
        let steps = game.valid_steps();
        if steps.is_empty() {
            break;
        }
        let i = agent.act(game).min(steps.len() - 1);
        if steps[i] == end_turn {
            turns += 1;
        }
        game.step(i);
        steps_taken += 1;
        if steps_taken > 3000 {
            return Outcome::Stall;
        }
    }
    if matches!(game.status, GameStatus::Defeat) || game.player.cur_hp <= 0 {
        Outcome::Loss
    } else {
        Outcome::Win {
            hp_lost: start_hp - game.player.cur_hp,
            turns,
        }
    }
}

fn eval_agent<A: CombatAgent>(
    name: &str,
    mut make_agent: impl FnMut() -> A,
    scenarios: &[(&str, &dyn Fn() -> Game)],
    trials: u32,
) {
    println!("\n=== {name} ===");
    let mut overall = Stats::default();
    for (sname, build) in scenarios {
        let mut s = Stats::default();
        for _ in 0..trials {
            let mut agent = make_agent();
            let mut game = build();
            s.trials += 1;
            overall.trials += 1;
            match play_out(&mut game, &mut agent) {
                Outcome::Win { hp_lost, turns } => {
                    s.wins += 1;
                    overall.wins += 1;
                    s.hp_lost_on_win += hp_lost.max(0) as u64;
                    s.turns_on_win += turns as u64;
                    overall.hp_lost_on_win += hp_lost.max(0) as u64;
                    overall.turns_on_win += turns as u64;
                }
                Outcome::Loss => {}
                Outcome::Stall => {
                    s.stalls += 1;
                    overall.stalls += 1;
                }
            }
        }
        print_row(sname, &s);
    }
    print_row("OVERALL", &overall);
}

fn print_row(name: &str, s: &Stats) {
    let wr = 100.0 * s.wins as f64 / s.trials.max(1) as f64;
    let avg_hp = if s.wins > 0 {
        s.hp_lost_on_win as f64 / s.wins as f64
    } else {
        0.0
    };
    let avg_turns = if s.wins > 0 {
        s.turns_on_win as f64 / s.wins as f64
    } else {
        0.0
    };
    println!(
        "  {name:<10} win {wr:5.1}%  avg hp lost {avg_hp:5.1}  avg turns {avg_turns:4.1}  stalls {}",
        s.stalls
    );
}

fn scenarios() -> Vec<(&'static str, Box<dyn Fn() -> Game>)> {
    use crate::game::GameBuilder;
    use crate::monsters::{
        cultist::Cultist, fungi_beast::FungiBeast, gremlin_nob::GremlinNob, jawworm::JawWorm,
        lagavulin::Lagavulin,
    };
    // A representative mid-Act-1 Ironclad deck so elites are actually winnable
    // and we measure agent skill rather than deck impossibility.
    fn base() -> GameBuilder {
        GameBuilder::default()
            .ironclad_starting_deck()
            .add_card(CardClass::Strike)
            .add_card(CardClass::Strike)
            .add_card(CardClass::TwinStrike)
            .add_card(CardClass::PommelStrike)
            .add_card(CardClass::Anger)
            .add_card(CardClass::ShrugItOff)
            .add_card(CardClass::Inflame)
    }
    vec![
        (
            "JawWorm",
            Box::new(|| base().build_combat_with_monster(JawWorm::new())),
        ),
        (
            "Cultist",
            Box::new(|| base().build_combat_with_monster(Cultist::new())),
        ),
        (
            "FungiBeast",
            Box::new(|| base().build_combat_with_monster(FungiBeast::new())),
        ),
        (
            "GremlinNob",
            Box::new(|| base().build_combat_with_monster(GremlinNob::new())),
        ),
        (
            "Lagavulin",
            Box::new(|| base().build_combat_with_monster(Lagavulin::new())),
        ),
    ]
}

#[test]
#[ignore]
fn player_eval() {
    let trials: u32 = std::env::var("EVAL_TRIALS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(2000);
    let built = scenarios();
    let scen: Vec<(&str, &dyn Fn() -> Game)> = built
        .iter()
        .map(|(n, f)| (*n, f.as_ref() as &dyn Fn() -> Game))
        .collect();

    eval_agent("RandomAgent", RandomAgent::new, &scen, trials);
    eval_agent("GreedyAgent", || GreedyAgent, &scen, trials);

    // The rollout agent is much slower, so it has its own (smaller) trial count.
    // Run with e.g. ROLLOUT_TRIALS=200 ROLLOUTS=16 cargo test --release player_eval ...
    if let Some(rt) = std::env::var("ROLLOUT_TRIALS")
        .ok()
        .and_then(|s| s.parse::<u32>().ok())
    {
        let rollouts: u32 = std::env::var("ROLLOUTS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(16);
        eval_agent(
            &format!("RolloutAgent(x{rollouts})"),
            || RolloutAgent::new(rollouts),
            &scen,
            rt,
        );
    }
}

#[test]
fn test_clone_for_search_isolated() {
    use crate::game::GameBuilder;
    use crate::monsters::gremlin_nob::GremlinNob;

    let mut game = GameBuilder::default()
        .ironclad_starting_deck()
        .add_card(CardClass::Inflame)
        .build_combat_with_monster(GremlinNob::new());

    let orig_hp = game.player.cur_hp;
    let orig_hand: Vec<CardClass> = game.hand.iter().map(|c| c.borrow().class).collect();
    let orig_monster_hp = game.monsters[0].creature.cur_hp;
    let orig_draw_len = game.draw_pile.len();

    // Fork and play the fork to the end of combat with a greedy policy.
    let mut sim = game.clone_for_search();
    let mut policy = GreedyAgent;
    let _ = play_out(&mut sim, &mut policy);

    // The original must be byte-for-byte untouched by the fork's mutations.
    assert_eq!(game.player.cur_hp, orig_hp);
    assert_eq!(game.monsters[0].creature.cur_hp, orig_monster_hp);
    assert_eq!(game.draw_pile.len(), orig_draw_len);
    let now_hand: Vec<CardClass> = game.hand.iter().map(|c| c.borrow().class).collect();
    assert_eq!(orig_hand, now_hand);

    // And the original is still a valid, playable game.
    assert!(!game.valid_steps().is_empty());
    game.step(0);
}
