use std::env;
use std::io::{self, BufRead, Write};

use yavalath_engine::engine::{outcome, Outcome, Position, SWAP_MOVE};

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

#[derive(Clone, Copy, Debug)]
enum Strategy {
    Random,
    DeterministicSequence,
    Strength(u8),
}

fn parse_arg_value(args: &[String], key: &str) -> Option<String> {
    args.windows(2)
        .find(|w| w[0] == key)
        .map(|w| w[1].clone())
}

fn deterministic_sequence_move(pos: Position) -> Option<u8> {
    const PREFERRED: [u8; 61] = [
        30, 22, 23, 29, 31, 37, 38, 15, 16, 17, 21, 24, 28, 32, 36, 39, 43, 44, 8, 9, 10, 11,
        14, 18, 20, 25, 27, 33, 35, 40, 42, 45, 46, 50, 51, 1, 2, 3, 4, 5, 7, 12, 13, 19, 26,
        34, 41, 47, 48, 49, 52, 53, 54, 0, 6, 55, 56, 57, 58, 59, 60,
    ];

    let us = pos.turn;
    for mv in legal_moves_with_swap(pos) {
        let Some((next, jp)) = apply_move(pos, mv) else {
            continue;
        };
        if matches!(outcome(next, jp), Outcome::Win(w, _) if w == us) {
            return Some(mv);
        }
    }

    for &mv in &PREFERRED {
        if !is_legal(pos, mv) {
            continue;
        }
        let Some((next, jp)) = apply_move(pos, mv) else {
            continue;
        };
        if matches!(outcome(next, jp), Outcome::Lose(l, _) if l == us) {
            continue;
        }
        return Some(mv);
    }

    legal_moves_with_swap(pos).into_iter().next()
}

fn is_legal(pos: Position, mv: u8) -> bool {
    if mv == SWAP_MOVE {
        pos.can_swap()
    } else {
        pos.occupied() & (1_u64 << mv) == 0
    }
}

fn legal_moves_with_swap(pos: Position) -> Vec<u8> {
    let mut moves = pos.legal_moves();
    if pos.can_swap() {
        moves.push(SWAP_MOVE);
    }
    moves
}

fn apply_move(pos: Position, mv: u8) -> Option<(Position, Option<(u8, u8)>)> {
    if mv == SWAP_MOVE {
        Some((pos.apply_swap()?, None))
    } else {
        Some((pos.apply(mv)?, Some((pos.turn, mv))))
    }
}

fn choose_move(pos: Position, strategy: Strategy, time_ms: f64, rng: &mut Rng64) -> Option<u8> {
    match strategy {
        Strategy::Random => {
            let legal = legal_moves_with_swap(pos);
            if legal.is_empty() {
                None
            } else {
                Some(legal[rng.gen_index(legal.len())])
            }
        }
        Strategy::DeterministicSequence => deterministic_sequence_move(pos),
        Strategy::Strength(strength) => {
            yavalath_engine::engine::best_move_with_strength(pos, time_ms, strength)
        }
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let strategy_name = parse_arg_value(&args, "--strategy").unwrap_or_else(|| "strength".into());
    let time_ms = parse_arg_value(&args, "--time-ms")
        .and_then(|s| s.parse::<f64>().ok())
        .unwrap_or(25.0);
    let strength = parse_arg_value(&args, "--strength")
        .and_then(|s| s.parse::<u8>().ok())
        .unwrap_or(2);
    let seed = parse_arg_value(&args, "--seed")
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(1);

    let strategy = match strategy_name.as_str() {
        "random" => Strategy::Random,
        "sequence" => Strategy::DeterministicSequence,
        "strength" => Strategy::Strength(strength),
        _ => Strategy::Strength(strength),
    };

    let stdin = io::stdin();
    let mut stdout = io::stdout();
    let mut rng = Rng64::new(seed);

    for line in stdin.lock().lines() {
        let Ok(board_hex) = line else { break };
        let board_hex = board_hex.trim();
        if board_hex.is_empty() {
            continue;
        }
        let Ok(pos) = yavalath_engine::engine::parse_board_hex(board_hex) else {
            let _ = writeln!(stdout, "-1");
            let _ = stdout.flush();
            continue;
        };
        let mv = choose_move(pos, strategy, time_ms, &mut rng)
            .map(i32::from)
            .unwrap_or(-1);
        let _ = writeln!(stdout, "{mv}");
        let _ = stdout.flush();
    }
}
