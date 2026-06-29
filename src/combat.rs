use rand::RngExt;

use crate::{
    actions::{
        discard_card::DiscardCardAction, draw::DrawAction,
        end_of_turn_discard::EndOfTurnDiscardAction, play_card::PlayCardAction,
        start_of_turn_energy::StartOfTurnEnergyAction,
    },
    creature::CreatureState,
    draw_pile::DrawPile,
    game::{CombatType, CreatureRef, Game, RareCardBaseChance, RunActionsGameState, UsePotionStep},
    map::RoomType,
    monster::Monster,
    monsters::{Combat, test::NoopMonster},
    potion::{Potion, random_potion_weighted},
    relic::RelicClass,
    rewards::{RewardType, Rewards, RewardsGameState},
    rng::rand_slice,
    state::{GameState, Steps},
    step::Step,
};

#[derive(Debug)]
pub struct RollCombatGameState;

impl GameState for RollCombatGameState {
    fn run(&self, game: &mut Game) {
        game.cur_room = Some(RoomType::Monster);
        if let Some(m) = game.force_monsters.take() {
            game.monsters = m;
        } else if game.roll_noop_monsters {
            game.monsters = vec![Monster::new(NoopMonster::new(), &mut game.rng)];
        } else {
            let num_easy_pool_combats = if game.is_in_act(1) { 3 } else { 2 };

            let mut combat;
            loop {
                combat = if game.num_combats_this_act < num_easy_pool_combats {
                    rand_slice(&mut game.rng, &game.easy_pool_combats)
                } else {
                    rand_slice(&mut game.rng, &game.hard_pool_combats)
                };
                if game.combat_history.len() >= 2
                    && game.combat_history[game.combat_history.len() - 2] == combat
                {
                    continue;
                }
                if game.combat_history.last() == Some(&combat) {
                    continue;
                }
                let invalid = match combat {
                    Combat::LargeSlime => game.combat_history.last() == Some(&Combat::SmallSlimes),
                    Combat::LotsOfSlimes => {
                        game.combat_history.last() == Some(&Combat::SmallSlimes)
                    }
                    Combat::ThreeLouses => game.combat_history.last() == Some(&Combat::TwoLouses),
                    Combat::ExordiumThugs => {
                        game.combat_history.last() == Some(&Combat::Looter)
                            || game.combat_history.last() == Some(&Combat::BlueSlaver)
                    }
                    Combat::RedSlaver => game.combat_history.last() == Some(&Combat::BlueSlaver),
                    Combat::ChosenAndByrd => {
                        game.combat_history.last() == Some(&Combat::ThreeByrds)
                            || game.combat_history.last() == Some(&Combat::Chosen)
                    }
                    Combat::ChosenAndCultist => game.combat_history.last() == Some(&Combat::Chosen),
                    Combat::SentryAndSphericGuardian => {
                        game.combat_history.last() == Some(&Combat::SphericGuardian)
                    }
                    Combat::ThreeDarklings => {
                        game.combat_history.last() == Some(&Combat::ThreeDarklings)
                    }
                    Combat::FourShapes => game.combat_history.last() == Some(&Combat::ThreeShapes),
                    _ => false,
                };
                if invalid {
                    continue;
                }
                break;
            }
            game.combat_history.push(combat);
            game.num_combats_this_act += 1;
            game.monsters = combat.monsters(game);
        }
        game.state.push_state(CombatBeginGameState(
            CombatType::Normal,
            RewardType::Monster,
        ));
    }
}

#[derive(Debug)]
pub struct RollEliteCombatGameState;

impl GameState for RollEliteCombatGameState {
    fn run(&self, game: &mut Game) {
        game.cur_room = Some(RoomType::Elite);
        let mut combat;
        loop {
            combat = rand_slice(&mut game.rng, &game.elites);
            if Some(combat) != game.last_elite {
                break;
            }
        }
        game.monsters = combat.monsters(game);
        game.last_elite = Some(combat);
        game.state
            .push_state(CombatBeginGameState(CombatType::Elite, RewardType::Elite));
    }
}

#[derive(Debug)]
pub struct RollBossCombatGameState;

impl GameState for RollBossCombatGameState {
    fn run(&self, game: &mut Game) {
        game.cur_room = Some(RoomType::Boss);
        game.monsters = game.boss.unwrap().monsters(game);
        game.state
            .push_state(CombatBeginGameState(CombatType::Boss, RewardType::Boss));
    }
}

#[derive(Debug)]
struct PlayerTurnEndGameState;

impl GameState for PlayerTurnEndGameState {
    fn run(&self, game: &mut Game) {
        if game.combat_finished() {
            game.state.push_state(CombatEndGameState);
            return;
        }
        game.should_add_extra_decay_status = true;
        game.trigger_relics_at_turn_end();
        game.player
            .trigger_statuses_turn_end(CreatureRef::player(), &mut game.action_queue);

        // trigger card end of turn effects
        let mut indexes_to_discard = Vec::new();
        let mut actions = vec![];
        for (i, c) in game.hand.iter().enumerate() {
            if let Some(a) = c.borrow().class.end_of_turn_in_hand_behavior() {
                indexes_to_discard.push(i);
                actions.push(a);
            }
        }
        for a in actions {
            a(game);
        }
        for i in indexes_to_discard.into_iter().rev() {
            game.action_queue
                .push_top(DiscardCardAction(game.hand.remove(i)));
        }

        game.action_queue.push_bot(EndOfTurnDiscardAction());
        game.state.push_state(MonsterTurnGameState);
        game.state.push_state(RunActionsGameState);
    }
}

#[derive(Debug)]
struct MonsterTurnGameState;

impl GameState for MonsterTurnGameState {
    fn run(&self, game: &mut Game) {
        assert!(game.monster_turn_queue_active.is_empty());
        if game.combat_finished() {
            game.state.push_state(CombatEndGameState);
            return;
        }
        for i in 0..game.monsters.len() {
            if !game.monsters[i].creature.is_actionable() {
                continue;
            }
            game.monsters[i].creature.start_of_turn_lose_block(false);
            game.monsters[i]
                .creature
                .trigger_statuses_turn_begin(CreatureRef::monster(i), &mut game.action_queue);
        }

        game.monster_turn_queue_active = game.monster_turn_queue_all.clone();

        game.state.push_state(EndOfRoundGameState);
        game.state.push_state(RunActionsGameState);
    }
}

#[derive(Debug)]
struct EndOfRoundGameState;

impl GameState for EndOfRoundGameState {
    fn run(&self, game: &mut Game) {
        if game.combat_finished() {
            game.state.push_state(CombatEndGameState);
            return;
        }
        game.should_add_extra_decay_status = false;
        for i in 0..game.monsters.len() {
            if !game.monsters[i].creature.is_actionable() {
                continue;
            }
            game.monsters[i]
                .creature
                .trigger_statuses_turn_end(CreatureRef::monster(i), &mut game.action_queue);
        }
        game.player
            .trigger_statuses_round_end(CreatureRef::player(), &mut game.action_queue);
        for (i, m) in game.monsters.iter_mut().enumerate() {
            m.creature
                .trigger_statuses_round_end(CreatureRef::monster(i), &mut game.action_queue);
        }
        game.state.push_state(PlayerTurnBeginGameState);
        game.state.push_state(RunActionsGameState);
        game.turn += 1;
    }
}

#[derive(Debug)]
struct CombatEndGameState;

impl GameState for CombatEndGameState {
    fn run(&self, game: &mut Game) {
        game.trigger_relics_at_combat_finish();
        game.state.push_state(RunActionsGameState);
    }
}

#[derive(Debug)]
struct ResetCombatGameState;

impl GameState for ResetCombatGameState {
    fn run(&self, game: &mut Game) {
        game.monsters.clear();
        game.player.clear_all_status();
        game.num_cards_played_this_turn = 0;
        game.monster_turn_queue_all.clear();
        game.should_add_extra_decay_status = false;
        game.chosen_cards.clear();
        game.cur_card.take();
        game.num_times_took_damage = 0;
        game.energy = 0;
        game.turn = 0;
        game.in_combat = CombatType::None;
        game.smoke_bombed = false;
        game.clear_all_piles();
    }
}

#[derive(Debug)]
struct RollCombatRewardsGameState(RewardType);

impl GameState for RollCombatRewardsGameState {
    fn clone_box(&self) -> Box<dyn GameState> {
        Box::new(RollCombatRewardsGameState(self.0))
    }
    fn run(&self, game: &mut Game) {
        if !game.smoke_bombed {
            let all_escaped = game
                .monsters
                .iter()
                .all(|c| matches!(c.creature.state, CreatureState::Escaped));
            let has_golden_idol = game.has_relic(RelicClass::GoldenIdol);
            match self.0 {
                RewardType::Monster => {
                    if !all_escaped {
                        let gold = game.rng.random_range(10..=20);
                        game.rewards.add_gold(gold, has_golden_idol);
                    }

                    let count = if game.has_relic(RelicClass::PrayerWheel) {
                        2
                    } else {
                        1
                    };
                    for _ in 0..count {
                        let cards = Rewards::gen_card_reward(game, RareCardBaseChance::Normal);
                        game.rewards.add_cards(cards);
                    }
                }
                RewardType::Mushrooms => {
                    let gold = game.rng.random_range(20..=30);
                    game.rewards.add_gold(gold, has_golden_idol);

                    let cards = Rewards::gen_card_reward(game, RareCardBaseChance::Normal);
                    game.rewards.add_cards(cards);

                    game.rewards.add_relic(RelicClass::OddMushroom);
                }
                RewardType::DeadAdventurer {
                    gold_reward,
                    relic_reward,
                } => {
                    let gold = game.rng.random_range(25..=35);
                    game.rewards.add_gold(gold, has_golden_idol);
                    if gold_reward {
                        game.rewards.add_gold(30, has_golden_idol);
                    }

                    let cards = Rewards::gen_card_reward(game, RareCardBaseChance::Normal);
                    game.rewards.add_cards(cards);

                    if relic_reward {
                        let r = game.next_relic_weighted();
                        game.rewards.add_relic(r);
                    }
                }
                RewardType::Elite => {
                    let gold = game.rng.random_range(25..=35);
                    game.rewards.add_gold(gold, has_golden_idol);

                    let cards = Rewards::gen_card_reward(game, RareCardBaseChance::Elite);
                    game.rewards.add_cards(cards);

                    let r = game.next_relic_weighted();
                    game.rewards.add_relic(r);

                    if game.has_relic(RelicClass::BlackStar) {
                        let mut r;
                        loop {
                            r = game.next_relic_weighted();
                            if !r.is_campfire_relic() {
                                break;
                            }
                        }
                        game.rewards.add_relic(r);
                    }
                }
                RewardType::Boss => {
                    let gold = game.rng.random_range(71..=79);
                    game.rewards.add_gold(gold, has_golden_idol);

                    let cards = Rewards::gen_card_reward(game, RareCardBaseChance::Boss);
                    game.rewards.add_cards(cards);
                }
            }

            if !all_escaped {
                if game.rng.random_range(0..100) < game.potion_chance
                    || game.has_relic(RelicClass::WhiteBeastStatue)
                {
                    game.potion_chance -= 10;
                    let p = random_potion_weighted(&mut game.rng);
                    game.rewards.add_potion(p);
                } else {
                    game.potion_chance += 10;
                }
            }
        }

        game.state.push_state(RewardsGameState);
        game.state.push_state(ResetCombatGameState);
    }
}

fn setup_combat_draw_pile(game: &mut Game) {
    let mut non_innate = Vec::new();
    let mut innate = Vec::new();
    for c in &game.master_deck {
        let c = game.clone_card_ref_same_id(c);
        if c.borrow().is_innate() || c.borrow().is_bottled {
            innate.push(c);
        } else {
            non_innate.push(c);
        }
    }
    let num_innate = innate.len() as i32;
    game.draw_pile = DrawPile::new(
        game.has_relic(RelicClass::FrozenEye),
        innate,
        non_innate,
        &mut game.rng,
    );
    let extra_draw = num_innate - game.draw_per_turn;
    if extra_draw > 0 {
        game.action_queue.push_bot(DrawAction(extra_draw));
    }
}

#[derive(Debug)]
pub struct CombatBeginGameState(pub CombatType, pub RewardType);

impl GameState for CombatBeginGameState {
    fn run(&self, game: &mut Game) {
        game.state.push_state(RollCombatRewardsGameState(self.1));

        assert!(!game.monsters.is_empty());

        game.in_combat = self.0;
        game.turn = 0;
        game.should_add_extra_decay_status = false;
        game.monster_turn_queue_all = game.get_alive_monsters();

        setup_combat_draw_pile(game);

        // player pre-combat relic setup
        game.trigger_relics_at_pre_combat();

        // monster pre-combat setup
        for i in 0..game.monsters.len() {
            if !game.monsters[i].creature.is_actionable() {
                continue;
            }
            game.monsters[i].behavior.pre_combat(
                &mut game.action_queue,
                CreatureRef::monster(i),
                &mut game.rng,
            );
        }

        game.state.push_state(PlayerTurnBeginGameState);
        game.state.push_state(RunActionsGameState);
    }
}

#[derive(Debug)]
struct PlayerTurnBeginGameState;

impl GameState for PlayerTurnBeginGameState {
    fn run(&self, game: &mut Game) {
        if game.combat_finished() {
            game.state.push_state(CombatEndGameState);
            return;
        }

        let info = game.calculate_monster_info();
        for i in 0..game.monsters.len() {
            if !game.monsters[i].creature.is_actionable() {
                continue;
            }
            game.monsters[i]
                .behavior
                .roll_next_action(&mut game.rng, &info);
        }

        game.num_cards_played_this_turn = 0;

        game.player
            .start_of_turn_lose_block(game.has_relic(RelicClass::Calipers));

        if game.turn == 0 {
            game.trigger_relics_at_combat_begin_pre_draw();
        }
        game.trigger_relics_at_turn_begin_pre_draw();
        game.player
            .trigger_statuses_turn_begin(CreatureRef::player(), &mut game.action_queue);

        game.action_queue.push_bot(DrawAction(game.draw_per_turn));

        if game.turn == 0 {
            game.trigger_relics_at_combat_begin_post_draw();
        }

        game.trigger_relics_at_turn_begin_post_draw();
        game.player
            .trigger_statuses_turn_begin_post_draw(CreatureRef::player(), &mut game.action_queue);

        game.action_queue.push_top(StartOfTurnEnergyAction());

        game.state.push_state(PlayerTurnGameState);
        game.state.push_state(RunActionsGameState);
    }
}

#[derive(Debug)]
struct PlayerTurnGameState;

impl GameState for PlayerTurnGameState {
    fn clone_box(&self) -> Box<dyn GameState> {
        Box::new(PlayerTurnGameState)
    }
    fn run(&self, game: &mut Game) {
        if game.combat_finished() {
            game.state.push_state(CombatEndGameState);
        }
    }
    fn valid_steps(&self, game: &Game) -> Option<Steps> {
        if game.combat_finished() {
            return None;
        }
        let mut moves = Steps::default();
        moves.push(EndTurnStep);
        for (ci, c) in game.hand.iter().enumerate() {
            if !game.can_play_card(&PlayCardAction::new(c.clone(), None, game)) {
                continue;
            }
            let c = c.borrow();
            if c.has_target() {
                for (mi, m) in game.monsters.iter().enumerate() {
                    if !m.creature.is_actionable() {
                        continue;
                    }
                    moves.push(PlayCardStep {
                        hand_index: ci,
                        target: Some(mi),
                    });
                }
            } else {
                moves.push(PlayCardStep {
                    hand_index: ci,
                    target: None,
                });
            }
        }
        for (pi, p) in game.potions.iter().enumerate() {
            if let Some(p) = p
                && p.can_use()
                && !p.can_use_outside_combat()
                && !(*p == Potion::Smoke && game.in_combat == CombatType::Boss)
            {
                if p.has_target() {
                    for (mi, m) in game.monsters.iter().enumerate() {
                        if !m.creature.is_actionable() {
                            continue;
                        }
                        moves.push(UsePotionStep {
                            potion_index: pi,
                            target: Some(mi),
                        });
                    }
                } else {
                    moves.push(UsePotionStep {
                        potion_index: pi,
                        target: None,
                    });
                }
            }
        }
        Some(moves)
    }
}

#[derive(Eq, PartialEq, Debug)]
pub struct EndTurnStep;

impl Step for EndTurnStep {
    fn should_pop_state(&self) -> bool {
        true
    }

    fn run(&self, game: &mut Game) {
        game.state.push_state(PlayerTurnEndGameState);
    }

    fn description(&self, _: &Game) -> String {
        "end turn".to_string()
    }
}

#[derive(Eq, PartialEq, Debug)]
pub struct PlayCardStep {
    pub hand_index: usize,
    pub target: Option<usize>,
}

impl Step for PlayCardStep {
    fn should_pop_state(&self) -> bool {
        false
    }

    fn run(&self, game: &mut Game) {
        let c = game.hand.remove(self.hand_index);
        let action = PlayCardAction::new(c, self.target.map(CreatureRef::monster), game);
        assert!(game.can_play_card(&action));
        game.card_queue.push(action);
        game.state.push_state(RunActionsGameState);
    }

    fn description(&self, game: &Game) -> String {
        let mut s = format!(
            "play card {} ({:?})",
            self.hand_index,
            game.hand[self.hand_index].borrow()
        );
        if let Some(t) = self.target {
            s += &format!(
                " on monster {} ({})",
                t,
                game.monster_str(CreatureRef::monster(t))
            );
        }
        s
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        actions::block::BlockAction,
        assert_matches, assert_not_matches,
        cards::{CardClass, CardCost},
        game::{AscendStep, DiscardPotionStep, GameBuilder, GameStatus},
        monsters::{
            Combat,
            looter::Looter,
            test::{AttackMonster, NoopMonster},
        },
        potion::Potion,
        rewards::RewardExitStep,
        status::Status,
    };

    use super::*;

    #[test]
    fn test_moves() {
        let mut g = GameBuilder::default()
            .build_combat_with_monsters(NoopMonster::new(), NoopMonster::new());
        g.add_card_to_hand(CardClass::DebugKill);
        g.add_card_to_hand(CardClass::Defend);
        g.add_potion(Potion::Fire);
        g.add_potion(Potion::Flex);
        assert_eq!(
            g.valid_steps(),
            vec![
                Box::new(EndTurnStep) as Box<dyn Step>,
                Box::new(PlayCardStep {
                    hand_index: 0,
                    target: Some(0),
                }),
                Box::new(PlayCardStep {
                    hand_index: 0,
                    target: Some(1),
                }),
                Box::new(PlayCardStep {
                    hand_index: 1,
                    target: None,
                }),
                Box::new(UsePotionStep {
                    potion_index: 0,
                    target: Some(0),
                }),
                Box::new(UsePotionStep {
                    potion_index: 0,
                    target: Some(1),
                }),
                Box::new(UsePotionStep {
                    potion_index: 1,
                    target: None,
                }),
                Box::new(DiscardPotionStep { potion_index: 0 }),
                Box::new(DiscardPotionStep { potion_index: 1 }),
            ]
        );
    }

    #[test]
    fn test_player_lose_block_start_of_turn() {
        let mut g = GameBuilder::default().build_combat();

        g.run_action(BlockAction::player_flat_amount(7));
        assert_eq!(g.player.block, 7);
        g.step_test(EndTurnStep);
        assert_eq!(g.player.block, 0);
    }

    #[test]
    fn test_monster_lose_block_start_of_turn() {
        let mut g = GameBuilder::default().build_combat();
        g.run_action(BlockAction::monster(CreatureRef::monster(0), 7));
        assert_eq!(g.monsters[0].creature.block, 7);
        g.step_test(EndTurnStep);
        assert_eq!(g.monsters[0].creature.block, 0);
    }

    #[test]
    fn test_barricade() {
        let mut g = GameBuilder::default()
            .add_monster_status(Status::Barricade, 1)
            .add_player_status(Status::Barricade, 1)
            .build_combat();
        g.run_action(BlockAction::monster(CreatureRef::player(), 7));
        g.run_action(BlockAction::monster(CreatureRef::monster(0), 7));
        g.step_test(EndTurnStep);
        assert_eq!(g.player.block, 7);
        assert_eq!(g.monsters[0].creature.block, 7);
    }

    #[test]
    fn test_unplayable_card_in_card_queue() {
        let mut g = GameBuilder::default()
            .add_player_status(Status::Entangled, 1)
            .build_combat();
        let c = g.new_card(CardClass::Thunderclap);
        g.card_queue.push(PlayCardAction::new(c, None, &g));
        g.run_all_actions();
        assert_eq!(g.discard_pile.len(), 1);
        assert_eq!(g.energy, 3);
    }

    #[test]
    fn test_free_to_play() {
        let mut g = GameBuilder::default().build_combat();
        let c = g.new_card(CardClass::Defend);
        match &mut c.borrow_mut().cost {
            CardCost::Cost {
                free_to_play_once, ..
            } => *free_to_play_once = true,
            _ => panic!(),
        }
        g.hand.push(c);
        assert_eq!(g.energy, 3);
        g.step_test(PlayCardStep {
            hand_index: 0,
            target: None,
        });
        assert_eq!(g.energy, 3);
        g.hand.push(g.discard_pile.pop().unwrap());
        g.step_test(PlayCardStep {
            hand_index: 0,
            target: None,
        });
        assert_eq!(g.energy, 2);

        g.step_test(EndTurnStep);
        assert_eq!(g.energy, 3);
        match &mut g.hand[0].borrow_mut().cost {
            CardCost::Cost {
                free_to_play_once, ..
            } => *free_to_play_once = true,
            _ => panic!(),
        }
        g.step_test(EndTurnStep);
        g.step_test(PlayCardStep {
            hand_index: 0,
            target: None,
        });
        assert_eq!(g.energy, 3);
    }

    #[test]
    fn test_has_relic() {
        use RelicClass::{BagOfPrep, BloodVial};
        let mut g = GameBuilder::default().build();

        assert!(!g.has_relic(BagOfPrep));
        assert!(!g.has_relic(BloodVial));

        g.add_relic(BagOfPrep);
        assert!(g.has_relic(BagOfPrep));
        assert!(!g.has_relic(BloodVial));

        g.remove_relic(BagOfPrep);
        assert!(!g.has_relic(BagOfPrep));
        assert!(!g.has_relic(BloodVial));
    }

    #[test]
    fn test_potions() {
        use Potion::{Attack, Skill};
        let mut g = GameBuilder::default().build();
        assert_eq!(g.potions, vec![None, None]);

        g.add_potion(Attack);
        assert_eq!(g.potions, vec![Some(Attack), None]);

        g.add_potion(Skill);
        assert_eq!(g.potions, vec![Some(Attack), Some(Skill)]);

        assert_eq!(g.take_potion(0), Attack);
        assert_eq!(g.potions, vec![None, Some(Skill)]);

        g.add_potion(Attack);
        assert_eq!(g.potions, vec![Some(Attack), Some(Skill)]);

        assert_eq!(g.take_potion(1), Skill);
        assert_eq!(g.potions, vec![Some(Attack), None]);
    }

    #[test]
    fn test_multi_attack_die_to_thorns() {
        let mut g = GameBuilder::default()
            .add_player_status(Status::Thorns, 999)
            .build_combat_with_monster(AttackMonster::with_attack_count(10, 10));
        g.player.cur_hp = 50;
        g.step_test(EndTurnStep);
        assert_eq!(g.player.cur_hp, 40);
    }

    #[test]
    fn test_card_queue_after_monsters_dead() {
        let mut g = GameBuilder::default().build_combat();
        g.add_card_to_draw_pile(CardClass::BandageUp);
        g.add_card_to_draw_pile(CardClass::DebugKill);
        g.add_card_to_draw_pile(CardClass::BandageUp);
        let hp = g.player.cur_hp;
        g.throw_potion(Potion::Chaos, None);
        assert_eq!(g.player.cur_hp, hp + 4);
    }

    #[test]
    fn test_defeat() {
        let mut g = GameBuilder::default().build_combat_with_monster(AttackMonster::new(999));
        g.step_test(EndTurnStep);
        assert_matches!(g.status, GameStatus::Defeat);
    }

    #[test]
    fn test_card_queue_duplicated_not_played() {
        let mut g = GameBuilder::default().build_combat();
        g.add_card_to_hand(CardClass::Normality);
        g.throw_potion(Potion::Duplication, None);
        g.throw_potion(Potion::Duplication, None);
        g.play_card(CardClass::Thunderclap, None);
        g.play_card(CardClass::Thunderclap, None);
        assert_eq!(g.discard_pile.len(), 2);
    }

    #[test]
    fn test_escaped_not_targetable() {
        let mut g =
            GameBuilder::default().build_combat_with_monsters(Looter::new(), NoopMonster::new());
        for _ in 0..5 {
            g.step_test(EndTurnStep);
        }
        g.play_card(CardClass::Thunderclap, None);
        assert_eq!(g.monsters[0].creature.cur_hp, g.monsters[0].creature.max_hp);
        assert_eq!(
            g.monsters[1].creature.cur_hp,
            g.monsters[1].creature.max_hp - 4
        );
    }

    #[test]
    fn test_act_1_combats() {
        for _ in 0..10 {
            let mut g = GameBuilder::default().build_with_rooms(&[
                RoomType::Monster,
                RoomType::Monster,
                RoomType::Monster,
                RoomType::Monster,
                RoomType::Monster,
                RoomType::Monster,
                RoomType::Monster,
                RoomType::Monster,
            ]);
            for i in 0..8 {
                g.step_test(AscendStep::new(0, i));
                g.play_card(CardClass::DebugKillAll, None);
                g.step_test(RewardExitStep);
            }
            assert_eq!(g.combat_history.len(), 8);
            for i in 0..3 {
                assert_matches!(
                    g.combat_history[i],
                    Combat::Cultist | Combat::TwoLouses | Combat::SmallSlimes | Combat::JawWorm
                );
            }
            for i in 3..8 {
                assert_not_matches!(
                    g.combat_history[i],
                    Combat::Cultist | Combat::TwoLouses | Combat::SmallSlimes | Combat::JawWorm
                );
            }
            for window in g.combat_history.windows(3) {
                assert_ne!(window[0], window[1]);
                assert_ne!(window[0], window[2]);
                assert_ne!(window[1], window[2]);
            }
            for window in g.combat_history.windows(2) {
                assert!(!(window[0] == Combat::TwoLouses && window[1] == Combat::ThreeLouses));
                assert!(!(window[0] == Combat::SmallSlimes && window[1] == Combat::LargeSlime));
                assert!(!(window[0] == Combat::SmallSlimes && window[1] == Combat::LotsOfSlimes));
            }
        }
    }

    #[test]
    fn test_act_1_elites() {
        let mut g = GameBuilder::default().build_with_rooms(&[
            RoomType::Elite,
            RoomType::Elite,
            RoomType::Elite,
            RoomType::Elite,
            RoomType::Elite,
            RoomType::Elite,
            RoomType::Elite,
            RoomType::Elite,
        ]);
        let mut last_name = "".to_owned();
        for i in 0..8 {
            g.step_test(AscendStep::new(0, i));
            assert_matches!(
                g.last_elite.unwrap(),
                Combat::Lagavulin | Combat::GremlinNob | Combat::ThreeSentries
            );
            assert_ne!(last_name, g.monsters[0].behavior.name());
            last_name = g.monsters[0].behavior.name().to_owned();
            g.play_card(CardClass::DebugKillAll, None);
            g.step_test(RewardExitStep);
        }
    }

    #[test]
    fn test_act_1_boss() {
        for _ in 0..5 {
            let mut g = GameBuilder::default().build_with_rooms(&[RoomType::Boss]);
            g.step_test(AscendStep::new(0, 0));
            match g.boss.unwrap() {
                Combat::Hexaghost => assert!(g.monsters[0].behavior.name().contains("hexa")),
                Combat::SlimeBoss => assert!(g.monsters[0].behavior.name().contains("slime")),
                Combat::Guardian => assert!(g.monsters[0].behavior.name().contains("guardian")),
                _ => panic!(),
            }
        }
    }
}
