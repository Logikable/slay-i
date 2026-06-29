//! Fork-only combat player: agents + an evaluation harness. Not for upstream.
//! Run with: cargo test --release player_eval -- --ignored --nocapture

#![cfg(test)]

use rand::RngExt;

use crate::{
    cards::{CardClass, CardCost, CardType},
    combat::{EndTurnStep, PlayCardStep},
    game::{CombatType, CreatureRef, Game, GameStatus, Rand},
    map::RoomType,
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
// Information-Set MCTS (open-loop UCT)
//
// Flat rollout spends its budget uniformly across every legal step; UCT spends
// it where it matters. We run N iterations: each samples a determinization
// (fork + fresh seed), descends the tree by UCB1, expands one node, rolls out
// to end of combat with Greedy, and backpropagates. Moves are keyed by a
// determinization-stable `MoveKey` (card class + upgrade + target, not the
// volatile hand index) so one tree is shared across all sampled worlds, the
// way ISMCTS handles hidden draw order / monster rolls.
// ---------------------------------------------------------------------------

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
enum MoveKey {
    EndTurn,
    Play {
        class: CardClass,
        upgraded: bool,
        target: Option<usize>,
    },
    // potion_index is stable across determinizations (potions aren't hidden),
    // so unlike a hand index it can key a shared tree branch directly.
    UsePotion {
        potion_index: usize,
        target: Option<usize>,
    },
}

// Legal moves at the current decision as (stable key, concrete step), deduped
// by key so interchangeable cards (e.g. two Strikes) collapse to one branch.
fn legal_moves(game: &Game) -> Vec<(MoveKey, Box<dyn Step>)> {
    let valid = game.valid_steps();
    let present = |s: &Box<dyn Step>| valid.iter().any(|v| v == s);
    let mut out: Vec<(MoveKey, Box<dyn Step>)> = vec![];
    let mut seen: std::collections::HashSet<MoveKey> = Default::default();
    let mut push = |out: &mut Vec<(MoveKey, Box<dyn Step>)>, k: MoveKey, s: Box<dyn Step>| {
        if seen.insert(k.clone()) {
            out.push((k, s));
        }
    };

    let et: Box<dyn Step> = Box::new(EndTurnStep);
    if present(&et) {
        push(&mut out, MoveKey::EndTurn, et);
    }
    for (ci, c) in game.hand.iter().enumerate() {
        let (class, upgraded, targeted) = {
            let b = c.borrow();
            (b.class, b.upgrade_count > 0, b.has_target())
        };
        let targets: Vec<Option<usize>> = if targeted {
            (0..game.monsters.len())
                .filter(|&mi| game.monsters[mi].creature.is_actionable())
                .map(Some)
                .collect()
        } else {
            vec![None]
        };
        for t in targets {
            let step: Box<dyn Step> = Box::new(PlayCardStep {
                hand_index: ci,
                target: t,
            });
            if present(&step) {
                push(
                    &mut out,
                    MoveKey::Play {
                        class,
                        upgraded,
                        target: t,
                    },
                    step,
                );
            }
        }
    }
    // Potions: try every (potion, target) and keep whatever is actually legal,
    // so ISMCTS can spend an emergency potion instead of hoarding it.
    for (pi, p) in game.potions.iter().enumerate() {
        if p.is_none() {
            continue;
        }
        let mut targets: Vec<Option<usize>> = vec![None];
        for mi in 0..game.monsters.len() {
            if game.monsters[mi].creature.is_actionable() {
                targets.push(Some(mi));
            }
        }
        for t in targets {
            let step: Box<dyn Step> = Box::new(crate::game::UsePotionStep {
                potion_index: pi,
                target: t,
            });
            if present(&step) {
                push(
                    &mut out,
                    MoveKey::UsePotion {
                        potion_index: pi,
                        target: t,
                    },
                    step,
                );
            }
        }
    }
    out
}

fn step_move(game: &mut Game, step: &Box<dyn Step>) {
    let i = game
        .valid_steps()
        .iter()
        .position(|s| s == step)
        .expect("chosen move must be legal in this determinization");
    game.step(i);
}

fn combat_over(game: &Game) -> bool {
    matches!(game.status, GameStatus::Defeat) || game.in_combat == CombatType::None
}

// Normalized terminal value in [0, 1]. A win always scores >= 0.5 (more so the
// more HP survives); a loss/stall scores by fraction of monster HP removed.
fn value01(game: &Game) -> f64 {
    let (mut cur, mut max) = (0i32, 0i32);
    for m in &game.monsters {
        cur += m.creature.cur_hp.max(0);
        max += m.creature.max_hp;
    }
    let progress = if max == 0 {
        1.0
    } else {
        1.0 - cur as f64 / max as f64
    };
    let won = !matches!(game.status, GameStatus::Defeat) && game.player.cur_hp > 0 && cur == 0;
    if won {
        0.5 + 0.5 * (game.player.cur_hp as f64 / game.player.max_hp.max(1) as f64)
    } else {
        0.45 * progress
    }
}

#[derive(Default)]
struct Edge {
    visits: f64,
    total: f64,
    avail: f64,
    child: Option<Box<MctsNode>>,
}

#[derive(Default)]
struct MctsNode {
    children: std::collections::HashMap<MoveKey, Edge>,
}

const UCB_C: f64 = 1.0;

fn mcts_iterate(
    node: &mut MctsNode,
    game: &mut Game,
    policy: &mut GreedyAgent,
    rng: &mut Rand,
) -> f64 {
    if combat_over(game) {
        return value01(game);
    }
    let moves = legal_moves(game);
    if moves.is_empty() {
        return value01(game);
    }
    for (k, _) in &moves {
        node.children.entry(k.clone()).or_default().avail += 1.0;
    }
    let untried: Vec<usize> = moves
        .iter()
        .enumerate()
        .filter(|(_, (k, _))| node.children[k].visits == 0.0)
        .map(|(i, _)| i)
        .collect();
    let chosen = if !untried.is_empty() {
        untried[rng.random_range(0..untried.len())]
    } else {
        let mut best = 0;
        let mut best_ucb = f64::MIN;
        for (i, (k, _)) in moves.iter().enumerate() {
            let e = &node.children[k];
            let ucb = e.total / e.visits + UCB_C * (e.avail.ln() / e.visits).sqrt();
            if ucb > best_ucb {
                best_ucb = ucb;
                best = i;
            }
        }
        best
    };
    let (key, step) = &moves[chosen];
    step_move(game, step);
    let expand = node.children[key].visits == 0.0;
    let value = if expand {
        let _ = play_out(game, policy);
        value01(game)
    } else {
        let child = node
            .children
            .get_mut(key)
            .unwrap()
            .child
            .get_or_insert_with(|| Box::new(MctsNode::default()));
        mcts_iterate(child, game, policy, rng)
    };
    let e = node.children.get_mut(key).unwrap();
    e.visits += 1.0;
    e.total += value;
    value
}

pub struct IsmctsAgent {
    iters: u32,
    rng: Rand,
}

impl IsmctsAgent {
    pub fn new(iters: u32) -> Self {
        Self {
            iters,
            rng: Rand::default(),
        }
    }
}

impl CombatAgent for IsmctsAgent {
    fn act(&mut self, game: &Game) -> usize {
        let moves = legal_moves(game);
        if moves.len() <= 1 {
            return 0;
        }
        let mut root = MctsNode::default();
        let mut policy = GreedyAgent;
        for _ in 0..self.iters {
            let mut sim = game.clone_for_search();
            let seed: u64 = self.rng.random();
            sim.rng = Rand::seed_from_u64(seed);
            mcts_iterate(&mut root, &mut sim, &mut policy, &mut self.rng);
        }
        // Robust child: the most-visited root move.
        let best_key = moves
            .iter()
            .map(|(k, _)| k)
            .max_by(|a, b| {
                let va = root.children.get(*a).map(|e| e.visits).unwrap_or(0.0);
                let vb = root.children.get(*b).map(|e| e.visits).unwrap_or(0.0);
                va.partial_cmp(&vb).unwrap()
            })
            .unwrap();
        let step = &moves.iter().find(|(k, _)| k == best_key).unwrap().1;
        game.valid_steps().iter().position(|s| s == step).unwrap()
    }
}

// ---------------------------------------------------------------------------
// Full-run driver: plays an entire Act-1 climb. Combat decisions go to a
// wrapped CombatAgent; every non-combat decision (Neow, map path, rewards,
// chest, campfire, deck edits, shop, events) is handled by simple heuristics.
// Steps are routed by their Debug type name and scored from public `Game`
// state, so the whole layer stays fork-only with no upstream `Step` changes.
// ---------------------------------------------------------------------------

// Leading identifier of a step's Debug output, e.g. "AscendStep",
// "CardRewardStep", "CampfireRestStep". Stable across field/variant values.
fn step_name(s: &Box<dyn Step>) -> String {
    let d = format!("{s:?}");
    d.split([' ', '{', '(']).next().unwrap_or("").to_owned()
}

fn indices_of(steps: &[Box<dyn Step>], name: &str) -> Vec<usize> {
    steps
        .iter()
        .enumerate()
        .filter(|(_, s)| step_name(s) == name)
        .map(|(i, _)| i)
        .collect()
}

fn first_of(steps: &[Box<dyn Step>], name: &str) -> Option<usize> {
    steps.iter().position(|s| step_name(s) == name)
}

fn has_kind(steps: &[Box<dyn Step>], name: &str) -> bool {
    steps.iter().any(|s| step_name(s) == name)
}

// Desirability of adding `class` to the deck (also reused as a keep score).
fn card_value(class: CardClass) -> i32 {
    use CardClass::*;
    match class {
        Strike | Defend => 1, // basics: we start with plenty, don't want more
        Bash => 6,
        Uppercut | Carnage | Hemokinesis => 7,
        Clothesline | TwinStrike | PommelStrike | Cleave => 5,
        Anger | IronWave | Headbutt | Clash | Thunderclap => 4,
        Inflame => 8,
        Armaments | ShrugItOff => 5,
        TrueGrit | Purity => 3,
        _ => match class.ty() {
            CardType::Power => 7,
            CardType::Attack => 5,
            CardType::Skill => 4,
            CardType::Status | CardType::Curse => -100,
        },
    }
}

// Higher = better to remove/transform out of the deck (curses, then basics).
fn removal_value(class: CardClass) -> i32 {
    let curse = if class.ty() == CardType::Curse {
        1000
    } else {
        0
    };
    let basic = match class {
        CardClass::Strike => 500,
        CardClass::Defend => 400,
        _ => 0,
    };
    curse + basic - card_value(class)
}

// Among `steps`, pick the one of kind `name` whose master-deck card maximizes
// `score`. valid_steps emits one step per master-deck card passing `pred`, in
// deck order, so the nth such step aligns with the nth matching deck card.
fn master_pick(
    game: &Game,
    steps: &[Box<dyn Step>],
    name: &str,
    pred: impl Fn(&crate::card::CardRef) -> bool,
    score: impl Fn(CardClass) -> i32,
) -> usize {
    let idxs = indices_of(steps, name);
    if idxs.is_empty() {
        return 0;
    }
    let mut best_ord = 0;
    let mut best = i32::MIN;
    let mut ord = 0;
    for c in game.master_deck.iter() {
        if pred(c) {
            let s = score(c.borrow().class);
            if s > best {
                best = s;
                best_ord = ord;
            }
            ord += 1;
        }
    }
    *idxs.get(best_ord).unwrap_or(&idxs[0])
}

pub struct FullRunAgent<C: CombatAgent> {
    combat: C,
}

impl<C: CombatAgent> FullRunAgent<C> {
    pub fn new(combat: C) -> Self {
        Self { combat }
    }

    fn hp_frac(game: &Game) -> f64 {
        game.player.cur_hp as f64 / game.player.max_hp.max(1) as f64
    }

    // Destination room type of an AscendStep, parsed from "ascend to (x, y)".
    fn ascend_room(game: &Game, s: &Box<dyn Step>) -> Option<RoomType> {
        let d = s.description(game);
        let inside = d.split('(').nth(1)?;
        let inside = inside.trim_end_matches(')');
        let mut it = inside.split(',');
        let x: usize = it.next()?.trim().parse().ok()?;
        let y: usize = it.next()?.trim().parse().ok()?;
        game.map.nodes.get(x)?.get(y)?.ty
    }

    // Pick the next map node by room type, biased by HP and deck strength.
    fn map_nav(&self, game: &Game, steps: &[Box<dyn Step>]) -> usize {
        let hp = Self::hp_frac(game);
        // Only fight elites once the deck can actually win them; a starter deck
        // walking into Lagavulin/Gremlin Nob is a guaranteed death.
        let deck_power: i32 = game
            .master_deck
            .iter()
            .map(|c| card_value(c.borrow().class))
            .sum();
        let elite_ok = hp > 0.8 && deck_power >= 45;
        let mut best = 0;
        let mut best_score = i32::MIN;
        for i in indices_of(steps, "AscendStep") {
            let score = match Self::ascend_room(game, &steps[i]) {
                Some(RoomType::Treasure) => 7,
                Some(RoomType::Monster) => 5,
                Some(RoomType::Shop) => 4,
                Some(RoomType::Event) => 3,
                Some(RoomType::Campfire) => {
                    if hp < 0.6 {
                        8
                    } else {
                        4
                    }
                }
                Some(RoomType::Elite) => {
                    if elite_ok {
                        6
                    } else {
                        -10
                    }
                }
                _ => 10, // boss / boss treasure: only option anyway
            };
            if score > best_score {
                best_score = score;
                best = i;
            }
        }
        best
    }

    // Neow blessing: thin the deck if offered, else take lasting value.
    fn blessing(&self, game: &Game, steps: &[Box<dyn Step>]) -> usize {
        let mut best = 0;
        let mut best_score = i32::MIN;
        for i in indices_of(steps, "ChooseBlessingStep") {
            let d = steps[i].description(game);
            let score = if d.contains("RemoveOne") {
                100
            } else if d.contains("CommonRelic") {
                80
            } else if d.contains("GainMaxHPSmall") {
                60
            } else if d.contains("TransformOne") {
                50
            } else if d.contains("RandomUncommonColorless") {
                45
            } else if d.contains("RandomPotion") {
                30
            } else if d.contains("RemoveRelic") {
                -100
            } else {
                0
            };
            if score > best_score {
                best_score = score;
                best = i;
            }
        }
        best
    }

    // Take everything good (gold, relics, potions if room), then the best card
    // in the pack if it clears the bar, else leave.
    fn rewards(&self, game: &Game, steps: &[Box<dyn Step>]) -> usize {
        if let Some(i) = first_of(steps, "GoldRewardStep") {
            return i;
        }
        if let Some(i) = first_of(steps, "StolenGoldRewardStep") {
            return i;
        }
        if let Some(i) = first_of(steps, "RelicRewardStep") {
            return i;
        }
        if game.potions.iter().any(|p| p.is_none())
            && let Some(i) = first_of(steps, "PotionRewardStep")
        {
            return i;
        }
        let card_idxs = indices_of(steps, "CardRewardStep");
        if !card_idxs.is_empty() {
            // CardRewardSteps are in (pack, card) row-major order, matching a
            // flatten of rewards.cards.
            let classes: Vec<CardClass> = game
                .rewards
                .cards
                .iter()
                .flatten()
                .map(|c| c.borrow().class)
                .collect();
            let mut best_k = 0;
            let mut best_v = i32::MIN;
            for (k, c) in classes.iter().enumerate() {
                if card_value(*c) > best_v {
                    best_v = card_value(*c);
                    best_k = k;
                }
            }
            if best_v >= 4 && best_k < card_idxs.len() {
                return card_idxs[best_k];
            }
        }
        first_of(steps, "RewardExitStep").unwrap_or(0)
    }

    fn campfire(&self, game: &Game, steps: &[Box<dyn Step>]) -> usize {
        if let Some(i) = first_of(steps, "CampfireDigStep") {
            return i; // free relic
        }
        let rest = first_of(steps, "CampfireRestStep");
        let upgrade = first_of(steps, "CampfireUpgradeStep");
        match (rest, upgrade) {
            (Some(r), Some(u)) => {
                if Self::hp_frac(game) < 0.55 {
                    r
                } else {
                    u
                }
            }
            (Some(r), None) => r,
            (None, Some(u)) => u,
            (None, None) => 0,
        }
    }

    // Events and anything unmodeled: prefer a safe exit, never burn a potion.
    fn generic(&self, game: &Game, steps: &[Box<dyn Step>]) -> usize {
        let is_potion = |s: &Box<dyn Step>| {
            matches!(step_name(s).as_str(), "UsePotionStep" | "DiscardPotionStep")
        };
        for (i, s) in steps.iter().enumerate() {
            if is_potion(s) {
                continue;
            }
            let d = s.description(game).to_lowercase();
            if ["leave", "ignore", "skip", "outrun", "refuse", "continue"]
                .iter()
                .any(|w| d.contains(w))
            {
                return i;
            }
        }
        steps.iter().position(|s| !is_potion(s)).unwrap_or(0)
    }
}

impl<C: CombatAgent> CombatAgent for FullRunAgent<C> {
    fn act(&mut self, game: &Game) -> usize {
        if game.in_combat != CombatType::None {
            return self.combat.act(game);
        }
        let steps = game.valid_steps();
        if steps.is_empty() {
            return 0;
        }
        if has_kind(&steps, "AscendStep") {
            return self.map_nav(game, &steps);
        }
        if has_kind(&steps, "ChooseBlessingStep") {
            return self.blessing(game, &steps);
        }
        if [
            "GoldRewardStep",
            "StolenGoldRewardStep",
            "RelicRewardStep",
            "PotionRewardStep",
            "CardRewardStep",
            "RewardExitStep",
            "SingingBowlStep",
            "SapphireKeyStep",
        ]
        .iter()
        .any(|k| has_kind(&steps, k))
        {
            return self.rewards(game, &steps);
        }
        if has_kind(&steps, "OpenChestStep") || has_kind(&steps, "SkipChestStep") {
            return first_of(&steps, "OpenChestStep").unwrap_or(0);
        }
        if [
            "CampfireRestStep",
            "CampfireUpgradeStep",
            "CampfireDigStep",
            "CampfireLiftStep",
            "CampfireTokeStep",
        ]
        .iter()
        .any(|k| has_kind(&steps, k))
        {
            return self.campfire(game, &steps);
        }
        if has_kind(&steps, "ChooseRemoveFromMasterStep") {
            return master_pick(
                game,
                &steps,
                "ChooseRemoveFromMasterStep",
                |c| c.borrow().can_remove_from_master_deck(),
                removal_value,
            );
        }
        if has_kind(&steps, "ChooseTransformMasterStep") {
            return master_pick(
                game,
                &steps,
                "ChooseTransformMasterStep",
                |c| c.borrow().can_remove_from_master_deck(),
                removal_value,
            );
        }
        if has_kind(&steps, "ChooseUpgradeMasterStep") {
            return master_pick(
                game,
                &steps,
                "ChooseUpgradeMasterStep",
                |c| c.borrow().can_upgrade(),
                card_value,
            );
        }
        if has_kind(&steps, "BossRewardChooseStep") {
            return first_of(&steps, "BossRewardChooseStep").unwrap();
        }
        if has_kind(&steps, "ShopExitStep") {
            return first_of(&steps, "ShopExitStep").unwrap();
        }
        self.generic(game, &steps)
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

    // ISMCTS, gated similarly:
    // ISMCTS_TRIALS=200 ISMCTS_ITERS=600 cargo test --release player_eval ...
    if let Some(it) = std::env::var("ISMCTS_TRIALS")
        .ok()
        .and_then(|s| s.parse::<u32>().ok())
    {
        let iters: u32 = std::env::var("ISMCTS_ITERS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(600);
        eval_agent(
            &format!("IsmctsAgent({iters})"),
            || IsmctsAgent::new(iters),
            &scen,
            it,
        );
    }
}

// Stops at the Act-1 boss reward (Act 2 monsters aren't implemented yet).
enum RunResult {
    Cleared { hp: i32, floor: i32 },
    Died { floor: i32 },
    Stalled,
}

fn play_full_run<A: CombatAgent>(game: &mut Game, agent: &mut A) -> RunResult {
    let mut steps_taken = 0;
    loop {
        match game.status {
            GameStatus::Defeat => return RunResult::Died { floor: game.floor },
            GameStatus::Victory => {
                return RunResult::Cleared {
                    hp: game.player.cur_hp,
                    floor: game.floor,
                };
            }
            GameStatus::Combat => {
                let steps = game.valid_steps();
                if steps.is_empty() {
                    return RunResult::Stalled;
                }
                // Act-1 boss is dead once its boss-treasure relic choice appears.
                if has_kind(&steps, "BossRewardChooseStep")
                    || has_kind(&steps, "BossRewardSkipStep")
                {
                    return RunResult::Cleared {
                        hp: game.player.cur_hp,
                        floor: game.floor,
                    };
                }
                let i = agent.act(game).min(steps.len() - 1);
                game.step(i);
                steps_taken += 1;
                if steps_taken > 100_000 {
                    return RunResult::Stalled;
                }
            }
        }
    }
}

#[derive(Default, Clone, Copy)]
struct RunStats {
    cleared: u32,
    hp_sum: i64,
    floor_sum: i64,
    deaths: u32,
    death_floor_sum: i64,
    stalls: u32,
    errors: u32,
    trials: u32,
}

impl RunStats {
    fn add(&mut self, r: RunResult) {
        self.trials += 1;
        match r {
            RunResult::Cleared { hp, floor } => {
                self.cleared += 1;
                self.hp_sum += hp as i64;
                self.floor_sum += floor as i64;
            }
            RunResult::Died { floor } => {
                self.deaths += 1;
                self.death_floor_sum += floor as i64;
            }
            RunResult::Stalled => self.stalls += 1,
        }
    }
    fn merge(&mut self, o: &RunStats) {
        self.cleared += o.cleared;
        self.hp_sum += o.hp_sum;
        self.floor_sum += o.floor_sum;
        self.deaths += o.deaths;
        self.death_floor_sum += o.death_floor_sum;
        self.stalls += o.stalls;
        self.errors += o.errors;
        self.trials += o.trials;
    }
}

// One worker: `trials` independent Act-1 runs. Each builds its own Game, so the
// non-Send Rc card graph never crosses a thread boundary.
fn run_trials(iters: u32, trials: u32) -> RunStats {
    use crate::game::GameBuilder;
    use crate::relic::RelicClass;
    let mut s = RunStats::default();
    for _ in 0..trials {
        // Isolate latent engine panics from rare card/monster combos so one bad
        // seed doesn't abort the whole sweep; count it and move on.
        let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let mut game = GameBuilder::default()
                .ironclad_starting_deck()
                .add_relic(RelicClass::BurningBlood)
                .build();
            if iters == 0 {
                let mut a = FullRunAgent::new(GreedyAgent);
                play_full_run(&mut game, &mut a)
            } else {
                let mut a = FullRunAgent::new(IsmctsAgent::new(iters));
                play_full_run(&mut game, &mut a)
            }
        }));
        match r {
            Ok(res) => s.add(res),
            Err(_) => {
                s.errors += 1;
                s.trials += 1;
            }
        }
    }
    s
}

fn eval_full_run(name: &str, iters: u32, trials: u32, threads: u32) {
    let threads = threads.max(1);
    let per = trials.div_ceil(threads);
    let mut total = RunStats::default();
    std::thread::scope(|scope| {
        let mut handles = vec![];
        let mut left = trials;
        for _ in 0..threads {
            let n = per.min(left);
            left -= n;
            if n > 0 {
                handles.push(scope.spawn(move || run_trials(iters, n)));
            }
        }
        for h in handles {
            total.merge(&h.join().unwrap());
        }
    });
    let t = total.trials;
    let pct = 100.0 * total.cleared as f64 / t.max(1) as f64;
    let div = |n: i64, d: u32| if d > 0 { n as f64 / d as f64 } else { 0.0 };
    println!("\n=== FullRunAgent({name}) over {t} Act-1 runs ===");
    println!("  cleared {}/{t} ({pct:.1}%)", total.cleared);
    println!(
        "  avg final hp on clear {:.1} (at floor {:.1})",
        div(total.hp_sum, total.cleared),
        div(total.floor_sum, total.cleared)
    );
    println!(
        "  deaths {} (avg death floor {:.1}), stalls {}, errors {}",
        total.deaths,
        div(total.death_floor_sum, total.deaths),
        total.stalls,
        total.errors
    );
}

#[test]
#[ignore]
fn full_run_eval() {
    let trials: u32 = std::env::var("FULLRUN_TRIALS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(200);
    // Combat is ISMCTS when FULLRUN_ISMCTS_ITERS>0, else the cheap Greedy policy.
    let iters: u32 = std::env::var("FULLRUN_ISMCTS_ITERS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    let threads: u32 = std::env::var("FULLRUN_THREADS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(12);
    // Caught per-trial panics are counted, not printed; keep stderr quiet.
    std::panic::set_hook(Box::new(|_| {}));
    let name = if iters == 0 {
        "Greedy".to_owned()
    } else {
        format!("ISMCTS({iters})")
    };
    eval_full_run(&name, iters, trials, threads);
}

#[test]
#[ignore]
fn full_run_trace() {
    use crate::game::GameBuilder;
    use crate::relic::RelicClass;

    let mut game = GameBuilder::default()
        .ironclad_starting_deck()
        .add_relic(RelicClass::BurningBlood)
        .build();
    let iters: u32 = std::env::var("FULLRUN_ISMCTS_ITERS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    let mut agent: Box<dyn CombatAgent> = if iters == 0 {
        Box::new(FullRunAgent::new(GreedyAgent))
    } else {
        Box::new(FullRunAgent::new(IsmctsAgent::new(iters)))
    };
    let mut steps_taken = 0;
    let mut prev_in_combat = false;
    let mut hp_at_combat_start = game.player.cur_hp;
    loop {
        if matches!(game.status, GameStatus::Defeat) {
            println!(
                "DIED floor {} room {:?} hp 0/{} deck {}",
                game.floor,
                game.cur_room,
                game.player.max_hp,
                game.master_deck.len()
            );
            break;
        }
        let steps = game.valid_steps();
        if steps.is_empty() || has_kind(&steps, "BossRewardChooseStep") {
            println!("CLEARED floor {} hp {}", game.floor, game.player.cur_hp);
            break;
        }
        let in_combat = game.in_combat != CombatType::None;
        if in_combat && !prev_in_combat {
            hp_at_combat_start = game.player.cur_hp;
            let ms: Vec<String> = game
                .monsters
                .iter()
                .map(|m| format!("{}({}hp)", m.behavior.name(), m.creature.cur_hp))
                .collect();
            println!(
                "floor {:>2} COMBAT start hp {} deck {}: {}",
                game.floor,
                game.player.cur_hp,
                game.master_deck.len(),
                ms.join(", ")
            );
        }
        if !in_combat && prev_in_combat {
            println!(
                "         COMBAT end   hp {} (lost {})",
                game.player.cur_hp,
                hp_at_combat_start - game.player.cur_hp
            );
        }
        prev_in_combat = in_combat;
        let i = agent.act(&game).min(steps.len() - 1);
        game.step(i);
        steps_taken += 1;
        if steps_taken > 100_000 {
            break;
        }
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
