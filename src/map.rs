use std::collections::HashSet;

use rand::{RngExt, seq::SliceRandom};

use crate::{game::Rand, rng::rand_slice};

#[derive(Clone)]
pub struct Map {
    pub nodes: Vec<Vec<Node>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RoomType {
    Monster,
    Elite,
    Event,
    Campfire,
    Shop,
    Treasure,
    Boss,
    BossTreasure,
}

impl RoomType {
    fn char(&self) -> char {
        match self {
            RoomType::Monster => 'm',
            RoomType::Elite => 'E',
            RoomType::Event => '?',
            RoomType::Campfire => '*',
            RoomType::Shop => '$',
            RoomType::Treasure => 'x',
            RoomType::Boss => 'B',
            RoomType::BossTreasure => 'X',
        }
    }
}

#[derive(Default, Clone)]
pub struct Node {
    pub ty: Option<RoomType>,
    // x of connected nodes above
    pub edges: Vec<usize>,
}

pub const MAP_WIDTH: usize = 7;
pub const MAP_HEIGHT: usize = 17;
const PRINT_WIDTH: usize = MAP_WIDTH * 2 - 1;
const PRINT_HEIGHT: usize = MAP_HEIGHT * 2 - 1;
const PRINT_TOTAL: usize = PRINT_WIDTH * PRINT_HEIGHT;

struct Print {
    strs: [char; PRINT_TOTAL],
}

impl Print {
    fn new() -> Self {
        Self { strs: [' '; _] }
    }
    fn str(&self) -> String {
        let mut ret = String::new();
        for y in (0..PRINT_HEIGHT).rev() {
            for x in 0..PRINT_WIDTH {
                ret.push(self.get(x, y));
            }
            ret.push('\n');
        }

        ret
    }
    fn idx(x: usize, y: usize) -> usize {
        y * PRINT_WIDTH + x
    }
    fn set(&mut self, x: usize, y: usize, c: char) {
        self.strs[Print::idx(x, y)] = c
    }
    fn get(&self, x: usize, y: usize) -> char {
        self.strs[Print::idx(x, y)]
    }
}

impl Default for Map {
    fn default() -> Self {
        Self {
            nodes: vec![vec![Node::default(); MAP_HEIGHT]; MAP_WIDTH],
        }
    }
}

impl Map {
    fn ancestor_depth(
        &self,
        mut y: usize,
        x1: usize,
        x2: usize,
        max_depth: usize,
    ) -> Option<usize> {
        assert_ne!(x1, x2);
        let mut depth = 0;
        let mut min_x = x1.min(x2);
        let mut max_x = x1.max(x2);
        while depth < max_depth && y != 0 {
            depth += 1;
            min_x = *self.parents(min_x, y).last().unwrap();
            max_x = *self.parents(max_x, y).first().unwrap();
            if min_x == max_x {
                return Some(depth);
            }
            y -= 1;
        }
        None
    }
    fn x_neighbors(x: usize) -> Vec<usize> {
        let mut ret = Vec::new();
        if x != 0 {
            ret.push(x - 1);
        }
        ret.push(x);
        if x != MAP_WIDTH - 1 {
            ret.push(x + 1);
        }
        ret
    }
    fn parents(&self, x: usize, y: usize) -> Vec<usize> {
        if y == 0 {
            return Vec::new();
        }
        let mut ret = Vec::new();
        for neighbor_x in Map::x_neighbors(x) {
            if self.nodes[neighbor_x][y - 1].edges.contains(&x) {
                ret.push(neighbor_x);
            }
        }
        ret
    }
    fn node_indexes(&self) -> Vec<(usize, usize)> {
        let mut ret = Vec::new();
        let mut cur_row = (0..MAP_WIDTH)
            .filter(|x| !self.nodes[*x][0].edges.is_empty())
            .collect::<Vec<_>>();
        for y in 0..MAP_HEIGHT {
            let mut next_row = HashSet::new();
            for x in cur_row {
                ret.push((x, y));
                for e in &self.nodes[x][y].edges {
                    next_row.insert(*e);
                }
            }
            cur_row = next_row.into_iter().collect::<Vec<_>>();
            cur_row.sort();
        }
        ret
    }
    pub fn print(&self) {
        print!("{}", self.str());
    }
    fn str(&self) -> String {
        let mut print = Print::new();
        for y in 0..MAP_HEIGHT {
            for x in 0..MAP_WIDTH {
                if !self.nodes[x][y].edges.is_empty() || !self.parents(x, y).is_empty() {
                    print.set(x * 2, y * 2, self.nodes[x][y].ty.unwrap().char());
                    for &e in &self.nodes[x][y].edges {
                        if e == x || y == MAP_HEIGHT - 3 || y == MAP_HEIGHT - 2 {
                            print.set(x * 2, y * 2 + 1, '|');
                        } else if e + 1 == x {
                            print.set(x * 2 - 1, y * 2 + 1, '\\');
                        } else if e == x + 1 {
                            print.set(x * 2 + 1, y * 2 + 1, '/');
                        } else {
                            panic!();
                        }
                    }
                }
            }
        }
        print.str()
    }
}

impl Map {
    fn generate_nodes(rng: &mut Rand, map: &mut Map) {
        let mut first_x = None;
        for _ in 0..6 {
            let mut cur_x = rng.random_range(0..MAP_WIDTH);
            // second starting x must be different from first starting x
            match first_x {
                Some(x) => {
                    while cur_x == x {
                        cur_x = rng.random_range(0..MAP_WIDTH);
                    }
                }
                None => first_x = Some(cur_x),
            }
            for y in 0..(MAP_HEIGHT - 3) {
                // choose one of three closest x values
                let mut next_x = rand_slice(rng, &Map::x_neighbors(cur_x));
                // reroll if ancestors are too close
                for p in map.parents(next_x, y + 1) {
                    if p == cur_x {
                        continue;
                    }
                    if let Some(a) = map.ancestor_depth(y, p, cur_x, 2)
                        && a <= 2
                    {
                        if next_x > cur_x {
                            if cur_x == 0 {
                                next_x = 0;
                            } else {
                                next_x = cur_x - rng.random_range(0..=1);
                            }
                        } else if next_x == cur_x {
                            next_x = rand_slice(rng, &Map::x_neighbors(cur_x));
                        } else if cur_x == MAP_WIDTH - 1 {
                            next_x = MAP_WIDTH - 1;
                        } else {
                            next_x = cur_x + rng.random_range(0..=1);
                        }
                    }
                }
                // adjust overlapping edges to not overlap
                if next_x > cur_x && map.nodes[cur_x + 1][y].edges.contains(&cur_x) {
                    next_x = cur_x;
                }
                if next_x < cur_x && map.nodes[cur_x - 1][y].edges.contains(&cur_x) {
                    next_x = cur_x;
                }
                map.nodes[cur_x][y].edges.push(next_x);
                cur_x = next_x;
            }
        }
        for x in 0..MAP_WIDTH {
            if !map.parents(x, MAP_HEIGHT - 3).is_empty() {
                map.nodes[x][MAP_HEIGHT - 3].edges.push(0);
            }
        }
        map.nodes[0][MAP_HEIGHT - 2].ty = Some(RoomType::Boss);
        map.nodes[0][MAP_HEIGHT - 2].edges.push(0);
        map.nodes[0][MAP_HEIGHT - 1].ty = Some(RoomType::BossTreasure);
    }
    fn generate_rooms_bag(rng: &mut Rand, map: &Map) -> Vec<RoomType> {
        let mut ret = Vec::new();
        let count1 = map
            .node_indexes()
            .iter()
            .filter(|(_, y)| *y != MAP_HEIGHT - 2)
            .count() as f32;
        let num_shops = (0.05 * count1) as usize;
        let num_campfires = (0.12 * count1) as usize;
        let num_events = (0.22 * count1) as usize;
        let num_elites = (0.08 * 1.6 * count1) as usize;
        for _ in 0..num_shops {
            ret.push(RoomType::Shop);
        }
        for _ in 0..num_campfires {
            ret.push(RoomType::Campfire);
        }
        for _ in 0..num_events {
            ret.push(RoomType::Event);
        }
        for _ in 0..num_elites {
            ret.push(RoomType::Elite);
        }
        let total_count = map
            .node_indexes()
            .iter()
            .filter(|(x, y)| map.nodes[*x][*y].ty.is_none())
            .count();
        while ret.len() < total_count {
            ret.push(RoomType::Monster);
        }
        ret.shuffle(rng);
        ret
    }
    fn is_valid_room(map: &Map, x: usize, y: usize, room: RoomType) -> bool {
        // campfires/elites cannot spawn in first 5 rows
        if y <= 4 && matches!(room, RoomType::Campfire | RoomType::Elite) {
            return false;
        }
        // row 14 is campfires, so no campfires in 13
        if y == 13 && room == RoomType::Campfire {
            return false;
        }
        // cannot have two campfires/elites/shops in a row
        if matches!(room, RoomType::Campfire | RoomType::Elite | RoomType::Shop) {
            for p_x in map.parents(x, y) {
                if let Some(parent_room) = map.nodes[p_x][y - 1].ty
                    && parent_room == room
                {
                    return false;
                }
            }
        }
        // cannot be same room type as a sibling
        for p_x in map.parents(x, y) {
            for &c_x in &map.nodes[p_x][y - 1].edges {
                if c_x == x {
                    continue;
                }
                if let Some(sibling_room_type) = map.nodes[c_x][y].ty
                    && sibling_room_type == room
                {
                    return false;
                }
            }
        }

        true
    }
    fn generate_rooms(rng: &mut Rand, map: &mut Map) {
        for x in 0..MAP_WIDTH {
            map.nodes[x][0].ty = Some(RoomType::Monster);
            map.nodes[x][8].ty = Some(RoomType::Treasure);
            map.nodes[x][MAP_HEIGHT - 3].ty = Some(RoomType::Campfire);
        }
        let mut rooms = Map::generate_rooms_bag(rng, map);
        for (x, y) in map.node_indexes() {
            if map.nodes[x][y].ty.is_some() {
                continue;
            }
            // choose first room from bag that's valid
            // TODO: skip over already checked room types
            let idx = rooms
                .iter()
                .position(|&room| Map::is_valid_room(map, x, y, room));
            if let Some(idx) = idx {
                let room = rooms.remove(idx);
                map.nodes[x][y].ty = Some(room);
            }
        }
        // mark unassigned rooms as monster
        // this can happen when the bag runs out of a specific room type early
        for (x, y) in map.node_indexes() {
            if map.nodes[x][y].ty.is_none() {
                map.nodes[x][y].ty = Some(RoomType::Monster);
            }
        }
    }
    pub fn generate(rng: &mut Rand) -> Self {
        let mut map = Map::default();
        Map::generate_nodes(rng, &mut map);
        Map::generate_rooms(rng, &mut map);
        map
    }
    pub fn straight_single_path(rooms: &[RoomType]) -> Self {
        let mut map = Map::default();
        for (i, room) in rooms.iter().enumerate() {
            map.nodes[0][i].ty = Some(*room);
            map.nodes[0][i].edges = vec![0];
        }
        map
    }
}

#[cfg(test)]
mod tests {
    use crate::assert_not_matches;

    use super::*;

    #[test]
    fn test_ancestor_depth() {
        let mut map = Map::default();
        // (0, 0) -> (0, 1)
        // (0, 0) -> (1, 1)
        // (0, 1) -> (0, 2)
        // (1, 1) -> (1, 2)
        map.nodes[0][0].edges = vec![0, 1];
        map.nodes[0][1].edges = vec![0];
        map.nodes[1][1].edges = vec![1];

        // (6, 0) -> (6, 1)
        // (6, 1) -> (6, 2)
        map.nodes[6][0].edges = vec![6];
        map.nodes[6][1].edges = vec![6];

        // (2, 1) -> (1, 2)
        // (2, 1) -> (2, 2)
        // (2, 1) -> (3, 2)
        // (3, 0) -> (2, 1)
        // (3, 0) -> (4, 1)
        // (4, 1) -> (3, 2)
        // (4, 1) -> (4, 2)
        // (4, 1) -> (5, 2)
        map.nodes[2][1].edges = vec![1, 2, 3];
        map.nodes[3][0].edges = vec![2, 4];
        map.nodes[4][1].edges = vec![3, 4, 5];

        assert_eq!(map.ancestor_depth(2, 1, 6, 99), None);
        assert_eq!(map.ancestor_depth(2, 3, 5, 99), Some(1));
        assert_eq!(map.ancestor_depth(2, 3, 4, 99), Some(1));
        assert_eq!(map.ancestor_depth(2, 2, 4, 99), Some(2));
        assert_eq!(map.ancestor_depth(2, 2, 4, 2), Some(2));
        assert_eq!(map.ancestor_depth(2, 2, 4, 1), None);
    }

    #[test]
    fn test_node_indexes() {
        let mut map = Map::default();
        // (0, 0) -> (0, 1)
        // (0, 0) -> (1, 1)
        // (0, 1) -> (0, 2)
        // (1, 1) -> (1, 2)
        map.nodes[0][0].edges = vec![0, 1];
        map.nodes[0][1].edges = vec![0];
        map.nodes[1][1].edges = vec![1];

        // (6, 0) -> (6, 1)
        // (6, 1) -> (6, 2)
        map.nodes[6][0].edges = vec![6];
        map.nodes[6][1].edges = vec![6];

        // (2, 1) -> (1, 2)
        // (2, 1) -> (2, 2)
        // (2, 1) -> (3, 2)
        // (3, 0) -> (2, 1)
        // (3, 0) -> (4, 1)
        // (4, 1) -> (3, 2)
        // (4, 1) -> (4, 2)
        // (4, 1) -> (5, 2)
        map.nodes[2][1].edges = vec![1, 2, 3];
        map.nodes[3][0].edges = vec![2, 4];
        map.nodes[4][1].edges = vec![3, 4, 5];

        assert_eq!(
            map.node_indexes(),
            vec![
                (0, 0),
                (3, 0),
                (6, 0),
                (0, 1),
                (1, 1),
                (2, 1),
                (4, 1),
                (6, 1),
                (0, 2),
                (1, 2),
                (2, 2),
                (3, 2),
                (4, 2),
                (5, 2),
                (6, 2)
            ]
        );
    }

    #[test]
    fn test_map() {
        let mut rng = Rand::default();
        for _ in 0..20 {
            let map = Map::generate(&mut rng);
            let num_start_points = (0..MAP_WIDTH)
                .filter(|&x| !map.nodes[x][0].edges.is_empty())
                .count();
            assert!(num_start_points >= 2);
            for y in 0..MAP_HEIGHT {
                for x in 0..(MAP_WIDTH - 1) {
                    // check no overlapping edges
                    assert!(
                        !(map.nodes[x][y].edges.contains(&(x + 1))
                            && map.nodes[x + 1][y].edges.contains(&x))
                    );
                }
            }

            map.print();
            for (x, y) in map.node_indexes() {
                let ty = map.nodes[x][y].ty.unwrap();
                match y {
                    0 => assert_eq!(ty, RoomType::Monster),
                    8 => assert_eq!(ty, RoomType::Treasure),
                    14 => assert_eq!(ty, RoomType::Campfire),
                    _ => {}
                }
                if y < 5 {
                    assert_not_matches!(ty, RoomType::Campfire | RoomType::Elite);
                }
                if matches!(
                    ty,
                    RoomType::Campfire | RoomType::Elite | RoomType::Shop | RoomType::Treasure
                ) {
                    for p_x in map.parents(x, y) {
                        assert_ne!(ty, map.nodes[p_x][y - 1].ty.unwrap());
                    }
                }
                if ty != RoomType::Monster && y != 8 && y != 14 {
                    for p_x in map.parents(x, y) {
                        for &e in &map.nodes[p_x][y - 1].edges {
                            if e != x {
                                assert_ne!(ty, map.nodes[e][y].ty.unwrap());
                            }
                        }
                    }
                }
            }
        }
    }
}
