use serde::Serialize;
use std::sync::OnceLock;

const BOARD_RADIUS: i32 = 4;
const BOARD_CELLS: usize = 61;
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
    let xy = cell_xy();
    let (x, y) = xy[idx as usize];
    (x * x + y * y).sqrt()
}

fn tactical_order(pos: Position) -> Vec<u8> {
    let mut moves = pos.legal_moves();
    moves.sort_by(|&a, &b| {
        let sa = move_priority(pos, a);
        let sb = move_priority(pos, b);
        sb.cmp(&sa).then_with(|| {
            distance_to_center(a)
                .partial_cmp(&distance_to_center(b))
                .unwrap_or(std::cmp::Ordering::Equal)
        })
    });
    moves
}

fn move_priority(pos: Position, mv: u8) -> i32 {
    let us = pos.turn;
    let them = us ^ 1;
    let Some(next) = pos.apply(mv) else {
        return -10_000;
    };
    match outcome(next, Some((us, mv))) {
        Outcome::Win(_, _) => return 20_000,
        Outcome::Lose(_, _) => return -20_000,
        _ => {}
    }
    let mut score = 0;
    for om in next.legal_moves() {
        let Some(reply) = next.apply(om) else {
            continue;
        };
        if matches!(outcome(reply, Some((them, om))), Outcome::Win(_, _)) {
            score -= 2_000;
        }
    }
    score - (distance_to_center(mv) * 100.0) as i32
}

fn heuristic(pos: Position, perspective: u8) -> i32 {
    let me = pos.stones(perspective);
    let them = pos.stones(perspective ^ 1);
    let mut score = 0;
    for &line in four_lines() {
        let mut me_count = 0;
        let mut them_count = 0;
        for &idx in &line {
            let mask = 1_u64 << idx;
            if me & mask != 0 {
                me_count += 1;
            }
            if them & mask != 0 {
                them_count += 1;
            }
        }
        if me_count > 0 && them_count > 0 {
            continue;
        }
        if me_count == 3 {
            score += 120;
        } else if me_count == 2 {
            score += 18;
        } else if me_count == 1 {
            score += 4;
        }
        if them_count == 3 {
            score -= 160;
        } else if them_count == 2 {
            score -= 24;
        } else if them_count == 1 {
            score -= 4;
        }
    }
    score
}

fn negamax(
    pos: Position,
    depth: i32,
    mut alpha: i32,
    beta: i32,
    root_player: u8,
    just_played: Option<(u8, u8)>,
) -> i32 {
    match outcome(pos, just_played) {
        Outcome::Win(w, _) => {
            return if w == root_player {
                100_000 + depth
            } else {
                -100_000 - depth
            }
        }
        Outcome::Lose(l, _) => {
            return if l == root_player {
                -100_000 - depth
            } else {
                100_000 + depth
            }
        }
        Outcome::Draw => return 0,
        Outcome::Invalid => return -200_000,
        Outcome::Ongoing => {}
    }
    if depth == 0 {
        return heuristic(pos, root_player);
    }

    let mut best = -1_000_000;
    let moves = tactical_order(pos);
    for mv in moves {
        let Some(next) = pos.apply(mv) else { continue };
        let score = -negamax(
            next,
            depth - 1,
            -beta,
            -alpha,
            root_player,
            Some((pos.turn, mv)),
        );
        if score > best {
            best = score;
        }
        if best > alpha {
            alpha = best;
        }
        if alpha >= beta {
            break;
        }
    }
    best
}

pub fn best_move(pos: Position, budget_ms: f64) -> Option<u8> {
    let legal = tactical_order(pos);
    if legal.is_empty() {
        return None;
    }
    let start = js_sys::Date::now();
    let root_player = pos.turn;
    let mut best = legal[0];
    let mut best_score = -1_000_000;
    let mut depth = 1;
    while js_sys::Date::now() - start < budget_ms.max(10.0) {
        let mut local_best = best;
        let mut local_best_score = -1_000_000;
        for &mv in &legal {
            if js_sys::Date::now() - start >= budget_ms.max(10.0) {
                break;
            }
            let Some(next) = pos.apply(mv) else { continue };
            let score = -negamax(
                next,
                depth - 1,
                -1_000_000,
                1_000_000,
                root_player,
                Some((root_player, mv)),
            );
            if score > local_best_score {
                local_best_score = score;
                local_best = mv;
            }
        }
        if local_best_score > best_score {
            best_score = local_best_score;
            best = local_best;
        }
        depth += 1;
        if depth > 6 {
            break;
        }
    }
    Some(best)
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
    if p0_count < p1_count || p0_count > p1_count + 1 {
        return Err("invalid move parity");
    }
    let ply = (p0_count + p1_count) as u8;
    let turn = if p0_count == p1_count { 0 } else { 1 };
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
}
