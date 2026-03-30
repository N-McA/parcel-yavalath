use serde::Serialize;
use std::sync::OnceLock;
#[cfg(not(target_arch = "wasm32"))]
use std::time::Instant;

const BOARD_RADIUS: i32 = 4;
const BOARD_CELLS: usize = 61;
pub const SWAP_MOVE: u8 = BOARD_CELLS as u8;
const DIRS: [(i32, i32); 6] = [(1, 0), (0, 1), (-1, 1), (-1, 0), (0, -1), (1, -1)];
const LINE_DIRS: [(i32, i32); 3] = [(1, 0), (0, 1), (1, -1)];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    Ongoing,
    Draw,
    Win(u8, [u8; 4]),
    Lose(u8, [u8; 3]),
    Invalid,
}

#[derive(Serialize)]
struct OutcomeResponse {
    state: &'static str,
    winner: Option<u8>,
    loser: Option<u8>,
    line: Vec<u8>,
}

#[derive(Clone, Copy, Debug)]
pub struct Position {
    pub p0: u64,
    pub p1: u64,
    pub turn: u8,
    pub ply: u8,
}

impl Position {
    #[cfg(any(test, target_arch = "wasm32"))]
    pub fn empty() -> Self {
        Self {
            p0: 0,
            p1: 0,
            turn: 0,
            ply: 0,
        }
    }

    pub fn occupied(self) -> u64 {
        self.p0 | self.p1
    }

    pub fn legal_moves(self) -> Vec<u8> {
        let mut moves = Vec::with_capacity(BOARD_CELLS);
        let occupied = self.occupied();
        for idx in 0..BOARD_CELLS as u8 {
            if occupied & (1_u64 << idx) == 0 {
                moves.push(idx);
            }
        }
        moves
    }

    pub fn can_swap(self) -> bool {
        self.ply == 1 && self.turn == 1
    }

    pub fn apply(self, mv: u8) -> Option<Self> {
        if mv as usize >= BOARD_CELLS {
            return None;
        }
        let mask = 1_u64 << mv;
        if self.occupied() & mask != 0 {
            return None;
        }
        let mut next = self;
        if self.turn == 0 {
            next.p0 |= mask;
        } else {
            next.p1 |= mask;
        }
        next.turn ^= 1;
        next.ply += 1;
        Some(next)
    }

    pub fn apply_swap(self) -> Option<Self> {
        if !self.can_swap() {
            return None;
        }
        let mut next = self;
        std::mem::swap(&mut next.p0, &mut next.p1);
        next.turn ^= 1;
        Some(next)
    }

    pub fn stones(self, player: u8) -> u64 {
        if player == 0 {
            self.p0
        } else {
            self.p1
        }
    }
}

fn axial_cells() -> Vec<(i32, i32)> {
    let mut cells = Vec::with_capacity(BOARD_CELLS);
    for q in -BOARD_RADIUS..=BOARD_RADIUS {
        let r_min = (-BOARD_RADIUS).max(-q - BOARD_RADIUS);
        let r_max = BOARD_RADIUS.min(-q + BOARD_RADIUS);
        for r in r_min..=r_max {
            cells.push((q, r));
        }
    }
    cells
}

fn idx_maps() -> (Vec<(i32, i32)>, std::collections::HashMap<(i32, i32), u8>) {
    let cells = axial_cells();
    let mut map = std::collections::HashMap::new();
    for (idx, c) in cells.iter().enumerate() {
        map.insert(*c, idx as u8);
    }
    (cells, map)
}

fn all_three_lines() -> Vec<[u8; 3]> {
    let (cells, map) = idx_maps();
    let mut lines = Vec::new();
    for &(q, r) in &cells {
        for &(dq, dr) in &LINE_DIRS {
            let s0 = (q, r);
            let s1 = (q + dq, r + dr);
            let s2 = (q + 2 * dq, r + 2 * dr);
            if let (Some(&a), Some(&b), Some(&c)) = (map.get(&s0), map.get(&s1), map.get(&s2)) {
                lines.push([a, b, c]);
            }
        }
    }
    lines.sort_unstable();
    lines.dedup();
    lines
}

fn all_four_lines() -> Vec<[u8; 4]> {
    let (cells, map) = idx_maps();
    let mut lines = Vec::new();
    for &(q, r) in &cells {
        for &(dq, dr) in &LINE_DIRS {
            let s0 = (q, r);
            let s1 = (q + dq, r + dr);
            let s2 = (q + 2 * dq, r + 2 * dr);
            let s3 = (q + 3 * dq, r + 3 * dr);
            if let (Some(&a), Some(&b), Some(&c), Some(&d)) =
                (map.get(&s0), map.get(&s1), map.get(&s2), map.get(&s3))
            {
                lines.push([a, b, c, d]);
            }
        }
    }
    lines.sort_unstable();
    lines.dedup();
    lines
}

fn three_lines() -> &'static [[u8; 3]] {
    static THREE_LINES: OnceLock<Vec<[u8; 3]>> = OnceLock::new();
    THREE_LINES.get_or_init(all_three_lines).as_slice()
}

fn four_lines() -> &'static [[u8; 4]] {
    static FOUR_LINES: OnceLock<Vec<[u8; 4]>> = OnceLock::new();
    FOUR_LINES.get_or_init(all_four_lines).as_slice()
}

fn stepped_neighbors() -> &'static [[[Option<u8>; 3]; 6]; BOARD_CELLS] {
    static STEPPED_NEIGHBORS: OnceLock<[[[Option<u8>; 3]; 6]; BOARD_CELLS]> = OnceLock::new();
    STEPPED_NEIGHBORS.get_or_init(|| {
        let (cells, map) = idx_maps();
        let mut stepped = [[[(None); 3]; 6]; BOARD_CELLS];
        for (idx, &(q, r)) in cells.iter().enumerate() {
            for (dir_idx, &(dq, dr)) in DIRS.iter().enumerate() {
                for step in 1..=3 {
                    let n = (q + dq * step, r + dr * step);
                    stepped[idx][dir_idx][(step - 1) as usize] = map.get(&n).copied();
                }
            }
        }
        stepped
    })
}

fn has_line3(bits: u64) -> Option<[u8; 3]> {
    for &l in three_lines() {
        if l.iter().all(|&i| bits & (1_u64 << i) != 0) {
            return Some(l);
        }
    }
    None
}

fn has_line4(bits: u64) -> Option<[u8; 4]> {
    for &l in four_lines() {
        if l.iter().all(|&i| bits & (1_u64 << i) != 0) {
            return Some(l);
        }
    }
    None
}

fn contiguous_in_direction(bits: u64, from: u8, dir_idx: usize) -> Vec<u8> {
    let mut run = Vec::with_capacity(3);
    for maybe_idx in stepped_neighbors()[from as usize][dir_idx] {
        let Some(idx) = maybe_idx else { break };
        if bits & (1_u64 << idx) == 0 {
            break;
        }
        run.push(idx);
    }
    run
}

fn window_containing(chain: &[u8], center_idx: usize, width: usize) -> Option<&[u8]> {
    if chain.len() < width {
        return None;
    }
    let start = center_idx.saturating_sub(width - 1);
    let max_start = center_idx.min(chain.len() - width);
    let start = start.min(max_start);
    Some(&chain[start..start + width])
}

fn has_line4_from_move(bits: u64, mv: u8) -> Option<[u8; 4]> {
    for &(dir_pos, dir_neg) in &[(0_usize, 3_usize), (1, 4), (2, 5)] {
        let mut negative = contiguous_in_direction(bits, mv, dir_neg);
        let positive = contiguous_in_direction(bits, mv, dir_pos);
        if negative.len() + 1 + positive.len() < 4 {
            continue;
        }
        negative.reverse();
        let center = negative.len();
        let mut chain = Vec::with_capacity(negative.len() + 1 + positive.len());
        chain.extend(negative);
        chain.push(mv);
        chain.extend(positive);
        if let Some(line) = window_containing(&chain, center, 4) {
            return line.try_into().ok();
        }
    }
    None
}

fn has_line3_from_move(bits: u64, mv: u8) -> Option<[u8; 3]> {
    for &(dir_pos, dir_neg) in &[(0_usize, 3_usize), (1, 4), (2, 5)] {
        let mut negative = contiguous_in_direction(bits, mv, dir_neg);
        let positive = contiguous_in_direction(bits, mv, dir_pos);
        if negative.len() + 1 + positive.len() < 3 {
            continue;
        }
        negative.reverse();
        let center = negative.len();
        let mut chain = Vec::with_capacity(negative.len() + 1 + positive.len());
        chain.extend(negative);
        chain.push(mv);
        chain.extend(positive);
        if let Some(line) = window_containing(&chain, center, 3) {
            return line.try_into().ok();
        }
    }
    None
}

pub fn outcome(pos: Position, just_played: Option<(u8, u8)>) -> Outcome {
    if let Some((player, last_move)) = just_played {
        let bits = pos.stones(player);
        if let Some(line4) = has_line4_from_move(bits, last_move) {
            return Outcome::Win(player, line4);
        }
        if let Some(line3) = has_line3_from_move(bits, last_move) {
            return Outcome::Lose(player, line3);
        }
    } else {
        if let Some(line4) = has_line4(pos.p0) {
            return Outcome::Win(0, line4);
        }
        if let Some(line3) = has_line3(pos.p0) {
            return Outcome::Lose(0, line3);
        }
        if let Some(line4) = has_line4(pos.p1) {
            return Outcome::Win(1, line4);
        }
        if let Some(line3) = has_line3(pos.p1) {
            return Outcome::Lose(1, line3);
        }
    }
    if pos.ply as usize >= BOARD_CELLS {
        return Outcome::Draw;
    }
    Outcome::Ongoing
}

fn cell_xy() -> Vec<(f64, f64)> {
    axial_cells()
        .into_iter()
        .map(|(q, r)| {
            let x = f64::from(q) + f64::from(r) / 2.0;
            let y = f64::from(r) * (3f64.sqrt() / 2.0);
            (x, y)
        })
        .collect()
}

fn distance_to_center(idx: u8) -> f64 {
    if idx as usize >= BOARD_CELLS {
        return f64::INFINITY;
    }
    let xy = cell_xy();
    let (x, y) = xy[idx as usize];
    (x * x + y * y).sqrt()
}

#[derive(Clone, Copy)]
struct SearchConfig {
    iterations_per_ms: f64,
    rollout_depth: usize,
    exploration: f64,
}

impl SearchConfig {
    fn from_strength(strength: u8) -> Self {
        match strength {
            0 => Self {
                iterations_per_ms: 1.0,
                rollout_depth: 16,
                exploration: 1.1,
            },
            1 => Self {
                iterations_per_ms: 2.0,
                rollout_depth: 24,
                exploration: 1.2,
            },
            2 => Self {
                iterations_per_ms: 4.0,
                rollout_depth: 32,
                exploration: 1.3,
            },
            _ => Self {
                iterations_per_ms: 7.0,
                rollout_depth: 42,
                exploration: 1.4,
            },
        }
    }
}

fn legal_moves_with_swap(pos: Position) -> Vec<u8> {
    let mut moves = pos.legal_moves();
    if pos.can_swap() {
        moves.push(SWAP_MOVE);
    }
    moves
}

fn apply_move_with_meta(pos: Position, mv: u8) -> Option<(Position, Option<(u8, u8)>)> {
    if mv == SWAP_MOVE {
        let next = pos.apply_swap()?;
        Some((next, None))
    } else {
        let next = pos.apply(mv)?;
        Some((next, Some((pos.turn, mv))))
    }
}

fn winner_from_outcome(result: Outcome) -> Option<u8> {
    match result {
        Outcome::Win(player, _) => Some(player),
        Outcome::Lose(player, _) => Some(player ^ 1),
        _ => None,
    }
}

fn losing_player_from_outcome(result: Outcome) -> Option<u8> {
    match result {
        Outcome::Lose(player, _) => Some(player),
        _ => None,
    }
}

fn immediate_winning_moves(pos: Position) -> Vec<u8> {
    let us = pos.turn;
    legal_moves_with_swap(pos)
        .into_iter()
        .filter(|&mv| {
            let Some((next, jp)) = apply_move_with_meta(pos, mv) else {
                return false;
            };
            matches!(outcome(next, jp), Outcome::Win(w, _) if w == us)
        })
        .collect()
}

fn immediate_losing_moves(pos: Position) -> Vec<u8> {
    let us = pos.turn;
    legal_moves_with_swap(pos)
        .into_iter()
        .filter(|&mv| {
            let Some((next, jp)) = apply_move_with_meta(pos, mv) else {
                return false;
            };
            matches!(outcome(next, jp), Outcome::Lose(l, _) if l == us)
        })
        .collect()
}

fn one_ply_safe_moves(pos: Position) -> Vec<u8> {
    let all = legal_moves_with_swap(pos);
    let us = pos.turn;
    all.into_iter()
        .filter(|&mv| {
            let Some((next, jp)) = apply_move_with_meta(pos, mv) else {
                return false;
            };
            if matches!(outcome(next, jp), Outcome::Lose(l, _) if l == us) {
                return false;
            }
            !next
                .legal_moves()
                .into_iter()
                .filter_map(|reply| {
                    let (reply_pos, reply_jp) = apply_move_with_meta(next, reply)?;
                    Some(outcome(reply_pos, reply_jp))
                })
                .any(|res| matches!(res, Outcome::Win(w, _) if w == next.turn))
        })
        .collect()
}

#[derive(Clone, Copy)]
struct Rng64 {
    state: u64,
}

impl Rng64 {
    fn new(seed: u64) -> Self {
        Self {
            state: if seed == 0 { 0x9e3779b97f4a7c15 } else { seed },
        }
    }

    fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state = x;
        x
    }

    fn gen_index(&mut self, len: usize) -> usize {
        if len <= 1 {
            0
        } else {
            (self.next_u64() as usize) % len
        }
    }
}

#[derive(Clone)]
struct Node {
    pos: Position,
    just_played: Option<(u8, u8)>,
    incoming_mv: Option<u8>,
    children: Vec<usize>,
    untried_moves: Vec<u8>,
    visits: u32,
    value_sum: f64,
}

fn centered_move_sort(moves: &mut [u8]) {
    moves.sort_by(|a, b| {
        distance_to_center(*a)
            .partial_cmp(&distance_to_center(*b))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
}

fn rollout_choice(pos: Position, rng: &mut Rng64) -> Option<u8> {
    let wins = immediate_winning_moves(pos);
    if !wins.is_empty() {
        let mut ordered = wins;
        centered_move_sort(&mut ordered);
        return ordered.first().copied();
    }

    let safe = one_ply_safe_moves(pos);
    if !safe.is_empty() {
        return Some(safe[rng.gen_index(safe.len())]);
    }

    let losing = immediate_losing_moves(pos);
    let mut all = legal_moves_with_swap(pos);
    if all.is_empty() {
        return None;
    }
    all.retain(|mv| !losing.contains(mv));
    if all.is_empty() {
        let mut fallback = legal_moves_with_swap(pos);
        centered_move_sort(&mut fallback);
        return fallback.first().copied();
    }
    Some(all[rng.gen_index(all.len())])
}

fn rollout(
    mut pos: Position,
    mut just_played: Option<(u8, u8)>,
    root_player: u8,
    rng: &mut Rng64,
    max_depth: usize,
) -> f64 {
    for _ in 0..max_depth {
        let result = outcome(pos, just_played);
        if let Some(winner) = winner_from_outcome(result) {
            return if winner == root_player { 1.0 } else { -1.0 };
        }
        if matches!(result, Outcome::Draw) {
            return 0.0;
        }

        let Some(mv) = rollout_choice(pos, rng) else {
            return 0.0;
        };
        let Some((next, next_jp)) = apply_move_with_meta(pos, mv) else {
            return 0.0;
        };
        pos = next;
        just_played = next_jp;
    }
    0.0
}

fn mcts_select_child(nodes: &[Node], node_idx: usize, c: f64) -> usize {
    let parent_visits = f64::from(nodes[node_idx].visits.max(1));
    let mut best_child = nodes[node_idx].children[0];
    let mut best_score = f64::NEG_INFINITY;
    for &child_idx in &nodes[node_idx].children {
        let child = &nodes[child_idx];
        if child.visits == 0 {
            return child_idx;
        }
        let exploit = child.value_sum / f64::from(child.visits);
        let explore = ((parent_visits.ln()) / f64::from(child.visits)).sqrt();
        let score = exploit + c * explore;
        if score > best_score {
            best_score = score;
            best_child = child_idx;
        }
    }
    best_child
}

pub fn best_move_with_strength(pos: Position, budget_ms: f64, strength: u8) -> Option<u8> {
    let legal = legal_moves_with_swap(pos);
    if legal.is_empty() {
        return None;
    }

    let wins = immediate_winning_moves(pos);
    if !wins.is_empty() {
        let mut ordered = wins;
        centered_move_sort(&mut ordered);
        return ordered.first().copied();
    }

    let safe = one_ply_safe_moves(pos);
    if !safe.is_empty() {
        if safe.len() == 1 {
            return safe.first().copied();
        }
    }

    let config = SearchConfig::from_strength(strength);
    let min_budget = 10.0;
    let adjusted_budget = budget_ms.max(min_budget);
    let max_iterations =
        ((adjusted_budget * config.iterations_per_ms) as usize).clamp(120, 250_000);
    let deadline = now_ms() + adjusted_budget;
    let root_player = pos.turn;
    let seed = pos.p0
        ^ pos.p1.rotate_left(7)
        ^ u64::from(pos.ply).rotate_left(17)
        ^ u64::from(strength).rotate_left(29);
    let mut rng = Rng64::new(seed);

    let mut root_untried = legal;
    centered_move_sort(&mut root_untried);
    let mut nodes = vec![Node {
        pos,
        just_played: None,
        incoming_mv: None,
        children: Vec::new(),
        untried_moves: root_untried,
        visits: 0,
        value_sum: 0.0,
    }];

    let mut iterations = 0usize;
    while iterations < max_iterations && now_ms() < deadline {
        iterations += 1;
        let mut node_idx = 0usize;
        let mut path = vec![0usize];

        while nodes[node_idx].untried_moves.is_empty() && !nodes[node_idx].children.is_empty() {
            node_idx = mcts_select_child(&nodes, node_idx, config.exploration);
            path.push(node_idx);
        }

        if !nodes[node_idx].untried_moves.is_empty() {
            let pick_idx = rng.gen_index(nodes[node_idx].untried_moves.len());
            let mv = nodes[node_idx].untried_moves.swap_remove(pick_idx);
            if let Some((next, jp)) = apply_move_with_meta(nodes[node_idx].pos, mv) {
                let mut untried = legal_moves_with_swap(next);
                centered_move_sort(&mut untried);
                let new_idx = nodes.len();
                nodes.push(Node {
                    pos: next,
                    just_played: jp,
                    incoming_mv: Some(mv),
                    children: Vec::new(),
                    untried_moves: untried,
                    visits: 0,
                    value_sum: 0.0,
                });
                nodes[node_idx].children.push(new_idx);
                node_idx = new_idx;
                path.push(node_idx);
            }
        }

        let leaf = &nodes[node_idx];
        let result = outcome(leaf.pos, leaf.just_played);
        let mut value = if let Some(winner) = winner_from_outcome(result) {
            if winner == root_player {
                1.0
            } else {
                -1.0
            }
        } else if matches!(result, Outcome::Draw) {
            0.0
        } else if let Some(loser) = losing_player_from_outcome(result) {
            if loser == root_player {
                -1.0
            } else {
                1.0
            }
        } else {
            rollout(
                leaf.pos,
                leaf.just_played,
                root_player,
                &mut rng,
                config.rollout_depth,
            )
        };

        for idx in path.into_iter().rev() {
            nodes[idx].visits += 1;
            nodes[idx].value_sum += value;
            value = -value;
        }
    }

    if nodes[0].children.is_empty() {
        let mut fallback = legal_moves_with_swap(pos);
        centered_move_sort(&mut fallback);
        return fallback.first().copied();
    }

    let mut best_child = nodes[0].children[0];
    for &c in &nodes[0].children[1..] {
        if nodes[c].visits > nodes[best_child].visits {
            best_child = c;
        }
    }
    nodes[best_child].incoming_mv
}

pub fn best_move(pos: Position, budget_ms: f64) -> Option<u8> {
    best_move_with_strength(pos, budget_ms, 2)
}

fn now_ms() -> f64 {
    #[cfg(target_arch = "wasm32")]
    {
        js_sys::Date::now()
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        static START: OnceLock<Instant> = OnceLock::new();
        START.get_or_init(Instant::now).elapsed().as_secs_f64() * 1000.0
    }
}

pub fn parse_board_hex(board_hex: &str) -> Result<Position, &'static str> {
    if board_hex.len() != 32 {
        return Err("board hex must be exactly 32 chars");
    }

    let mut bits = String::with_capacity(128);
    for ch in board_hex.chars() {
        let Some(v) = ch.to_digit(16) else {
            return Err("invalid hex");
        };
        bits.push(if v & 0b1000 != 0 { '1' } else { '0' });
        bits.push(if v & 0b0100 != 0 { '1' } else { '0' });
        bits.push(if v & 0b0010 != 0 { '1' } else { '0' });
        bits.push(if v & 0b0001 != 0 { '1' } else { '0' });
    }

    let p0_bits = &bits[0..64];
    let p1_bits = &bits[64..128];

    let mut p0: u64 = 0;
    let mut p1: u64 = 0;
    for (i, ch) in p0_bits.chars().rev().enumerate() {
        if ch == '1' {
            p0 |= 1_u64 << i;
        }
    }
    for (i, ch) in p1_bits.chars().rev().enumerate() {
        if ch == '1' {
            p1 |= 1_u64 << i;
        }
    }

    if p0 & p1 != 0 {
        return Err("players overlap on occupied cells");
    }

    let p0_count = p0.count_ones();
    let p1_count = p1.count_ones();
    let is_swapped_opening = p0_count == 0 && p1_count == 1;
    if !(is_swapped_opening || (p0_count >= p1_count && p0_count <= p1_count + 1)) {
        return Err("invalid move parity");
    }
    let ply = (p0_count + p1_count) as u8;
    let turn = if is_swapped_opening || p0_count == p1_count {
        0
    } else {
        1
    };
    Ok(Position { p0, p1, turn, ply })
}

pub fn encode_outcome(outcome: Outcome) -> String {
    let payload = match outcome {
        Outcome::Ongoing => OutcomeResponse {
            state: "ongoing",
            winner: None,
            loser: None,
            line: vec![],
        },
        Outcome::Draw => OutcomeResponse {
            state: "draw",
            winner: None,
            loser: None,
            line: vec![],
        },
        Outcome::Win(player, line) => OutcomeResponse {
            state: "win",
            winner: Some(player),
            loser: None,
            line: line.to_vec(),
        },
        Outcome::Lose(player, line) => OutcomeResponse {
            state: "lose",
            winner: Some(player ^ 1),
            loser: Some(player),
            line: line.to_vec(),
        },
        Outcome::Invalid => OutcomeResponse {
            state: "invalid",
            winner: None,
            loser: None,
            line: vec![],
        },
    };
    serde_json::to_string(&payload).unwrap_or_else(|_| "{\"state\":\"invalid\"}".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn position_after_moves(moves: &[u8]) -> Position {
        let mut pos = Position::empty();
        for &mv in moves {
            pos = pos.apply(mv).expect("move must be legal");
        }
        pos
    }

    #[test]
    fn empty_board_parses() {
        let p = parse_board_hex("00000000000000000000000000000000").unwrap();
        assert_eq!(p.ply, 0);
        assert_eq!(p.turn, 0);
    }

    #[test]
    fn legal_move_count() {
        let p = Position::empty();
        assert_eq!(p.legal_moves().len(), 61);
    }

    #[test]
    fn swapped_opening_parses() {
        // Only P1 has one stone: valid board after swap choice.
        let mut p1: u64 = 0;
        p1 |= 1_u64 << 30;
        let bits = format!("{:064b}{:064b}", 0_u64, p1)
            .chars()
            .collect::<Vec<_>>();
        let mut hex = String::new();
        for chunk in bits.chunks(4) {
            let s: String = chunk.iter().collect();
            hex.push(std::char::from_digit(u32::from_str_radix(&s, 2).unwrap(), 16).unwrap());
        }
        let p = parse_board_hex(&hex).unwrap();
        assert_eq!(p.ply, 1);
        assert_eq!(p.turn, 0);
    }

    #[test]
    fn swap_is_legal_only_for_second_player_after_first_move() {
        let p = position_after_moves(&[30]);
        assert!(p.can_swap());
        let next = p.apply_swap().unwrap();
        assert_eq!(next.p0, 0);
        assert_eq!(next.p1, 1_u64 << 30);
        assert_eq!(next.turn, 0);
        assert!(!next.can_swap());
    }

    #[test]
    fn engine_takes_immediate_win() {
        // Player 0 can win immediately with 50:
        // 26-35-43-50 is a line of four on r = -4.
        let pos = position_after_moves(&[26, 0, 35, 1, 43, 5]);
        assert_eq!(pos.turn, 0);
        assert_eq!(best_move(pos, 200.0), Some(50));
    }

    #[test]
    fn engine_blocks_opponents_immediate_win() {
        // Player 1 threatens 26-35-43-50; only 50 blocks the immediate win.
        let pos = position_after_moves(&[0, 26, 10, 35, 20, 43]);
        assert_eq!(pos.turn, 0);
        assert_eq!(best_move(pos, 200.0), Some(50));
    }

    #[test]
    fn best_move_handles_swap_position_without_panicking() {
        let pos = position_after_moves(&[30]);
        assert!(pos.can_swap());
        let mv = best_move_with_strength(pos, 60.0, 2);
        assert!(mv.is_some());
    }
}
