extern crate rand;
use rand::{thread_rng, Rng};

// extern crate time;
// use time::Duration;
// use time::PreciseTime;

extern crate ordered_float;
use ordered_float::OrderedFloat;

extern crate smallvec;
use smallvec::SmallVec;

extern crate quicksort;
use quicksort::quicksort;

extern crate serde;
extern crate serde_json;

use std::collections::HashMap;
use std::fmt;
use std::hash::Hash;
use std::io::stdout;
use std::io::Write;

mod constants;

static EXCLUDE_TOP_BITS: u64 = !(!0 << 61);
static N_ROLLOUTS: u32 = 1_000;
static C: f32 = 0.75;
static DISCOUNT: f32 = 0.975;
const CANON_LIMIT: u32 = 5;

static WIN_VALUE: f32 = 1f32;
static LOSE_VALUE: f32 = -1f32;
static DRAW_VALUE: f32 = 0f32;

static SHOW_MATCHUPS: bool = false;
static VERBOSE_PERF: bool = true;

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum GameOutcome {
    Winner(Player),
    Ongoing,
    Invalid,
    Draw,
}

#[derive(Debug, PartialEq, Clone, Copy, Hash, Eq, PartialOrd, Ord, Serialize, Deserialize)]
enum Player {
    P0,
    P1,
}

type SomeMoves<M> = SmallVec<[M; 64]>;
type SomeBoards<M> = SmallVec<[M; 64]>;
type SomeIdxs = SmallVec<[u8; 64]>;
type TransTable<B> = HashMap<B, (f32, f32)>;

trait GameMove: Copy + Clone + Eq + fmt::Display {}

trait GameBoard: Copy + Clone + Hash + Eq + fmt::Display {
    type Move: GameMove;
    fn new() -> Self;
    fn outcome(&self) -> GameOutcome;
    fn whose_turn(&self) -> Player;
    fn apply_move(&self, m: Self::Move) -> Self;
    fn legal_moves(&self) -> SomeMoves<Self::Move>;
    fn afterstates(&self) -> SomeBoards<Self>;
    fn unreachable_from(&self, state: Self) -> bool;
    fn canonical(&self) -> Self;
}

trait GameAgent {
    type Board: GameBoard;
    fn choose_move(&mut self, b: Self::Board, terminate: &mut (FnMut() -> bool)) -> <Self::Board as GameBoard>::Move;
    fn reset_game_specific_state(&mut self);
}

struct MCTS<B>
where
    B: GameBoard,
{
    transposition_table: TransTable<B>,
    n_rollouts: u32,
}

impl<B> MCTS<B>
where
    B: GameBoard,
{
    fn new(n_rollouts: u32) -> Self {
        return MCTS {
            n_rollouts,
            transposition_table: HashMap::new(),
        };
    }
    fn is_expanded(&self, state: B) -> bool {
        for s in state.afterstates() {
            if !self.transposition_table.contains_key(&s) {
                return false;
            }
        }
        return true;
    }
    fn ucb_choice(&self, state: B) -> B::Move {
        let parent_visits;
        let v = self.transposition_table.get(&state);
        match v {
            None => parent_visits = 1f32,
            Some((n, _)) => parent_visits = *n,
        };
        let player_sign;
        match state.whose_turn() {
            Player::P0 => player_sign = 1f32,
            Player::P1 => player_sign = -1f32,
        };
        let score = |&m| {
            let bb = state.apply_move(m);
            let (child_visits, child_wins) = self.transposition_table.get(&bb).unwrap();
            let winrate = player_sign * child_wins / child_visits;
            let v = winrate + C * (2.0 * parent_visits.ln() / (child_visits)).sqrt();
            return OrderedFloat(v);
        };
        let ms = state.legal_moves();
        return *ms.iter().max_by_key(|&m| score(m)).unwrap();
    }
    fn do_random_rollouts(&mut self, root_state: B, n: u32) {
        let mut states = Vec::new();
        let mut outcome_value;
        let whoami = root_state.whose_turn();
        use GameOutcome::*;
        use Player::*;
        for _ in 0..n {
            states.clear();
            states.push(root_state);
            loop {
                let state = *states.last().unwrap();
                match state.outcome() {
                    GameOutcome::Ongoing => {
                        let next_states = state.afterstates();
                        // Don't play losing moves.
                        let not_loses = next_states
                            .iter()
                            .filter(|ns| match (whoami, ns.outcome()) {
                                (P0, Winner(P1)) => false,
                                (P1, Winner(P0)) => false,
                                _ => true,
                            })
                            .collect::<SomeBoards<&B>>();
                        let wins = next_states
                            .iter()
                            .filter(|ns| match (whoami, ns.outcome()) {
                                (P0, Winner(P0)) => true,
                                (P1, Winner(P1)) => true,
                                _ => false,
                            })
                            .collect::<SomeBoards<&B>>();
                        match wins.first() {
                            Some(next_state) => states.push(**next_state),
                            None => {
                                let next_state;
                                if not_loses.len() > 0 {
                                    next_state =
                                        *uniform_random_choice(&mut not_loses.iter()).unwrap();
                                    states.push(*next_state)
                                } else {
                                    next_state =
                                        uniform_random_choice(&mut next_states.iter()).unwrap();
                                    states.push(*next_state)
                                }
                            }
                        }
                    }
                    Winner(P1) => {
                        outcome_value = LOSE_VALUE;
                        break;
                    }
                    Winner(P0) => {
                        outcome_value = WIN_VALUE;
                        break;
                    }
                    Draw => {
                        outcome_value = DRAW_VALUE;
                        break;
                    }
                    Invalid => panic!(),
                }
            }
            for &state in states.iter() {
                let (visitation_count, win_count) = self
                    .transposition_table
                    .entry(state)
                    .or_insert((0f32, 0f32));
                *visitation_count += 1f32;
                *win_count += outcome_value;
            }
        }
    }
    fn clean_table(&mut self, root_state: B) {
        if VERBOSE_PERF {
            println!(
                "size of the transposition table before clean {}",
                self.transposition_table.len()
            );
            // let start = PreciseTime::now();
            self.transposition_table
                .retain(|state, _| !state.unreachable_from(root_state));
            // let end = PreciseTime::now();
            // println!("{} seconds to clean table", start.to(end));
            // println!(
            //     "size of the transposition table after clean: {}",
            //     self.transposition_table.len()
            // );
        } else {
            self.transposition_table
                .retain(|state, _| !state.unreachable_from(root_state));
        }
    }
}

impl<B> GameAgent for MCTS<B>
where
    B: GameBoard,
{
    type Board = B;
    fn choose_move(&mut self, root_state: B, terminate: &mut (FnMut() -> bool)) -> B::Move {
        self.clean_table(root_state);
        let mut states = Vec::new();
        let mut outcome_value;
        // let start = Date::now();
        let mut achieved_rollouts = 0;
        for _rollout_n in 0..self.n_rollouts {
            if _rollout_n % 10 == 0 {
                print!("\r{}", _rollout_n);
                stdout().flush();
            }
            if terminate() { break; } 
            achieved_rollouts += 1;
            states.clear();
            states.push(root_state);
            loop {
                let state = *states.last().unwrap();
                match state.outcome() {
                    GameOutcome::Ongoing => {
                        if self.is_expanded(state) {
                            let m = self.ucb_choice(state);
                            states.push(state.apply_move(m));
                        } else {
                            for next_state in state.afterstates() {
                                self.do_random_rollouts(next_state, 1)
                            }
                        }
                    }
                    GameOutcome::Winner(Player::P1) => {
                        outcome_value = LOSE_VALUE;
                        break;
                    }
                    GameOutcome::Winner(Player::P0) => {
                        outcome_value = WIN_VALUE;
                        break;
                    }
                    GameOutcome::Draw => {
                        outcome_value = DRAW_VALUE;
                        break;
                    }
                    GameOutcome::Invalid => panic!(),
                }
            }
            for (depth, &state) in states.iter().enumerate() {
                let (visitation_count, win_count) = self
                    .transposition_table
                    .entry(state)
                    .or_insert((0f32, 0f32));
                *visitation_count += 1f32;
                *win_count += outcome_value * DISCOUNT.powi(depth as i32);
            }
        }
        println!("");
        let score = |&mv| {
            // For picking an actual move, we pick by visit count!
            // Thus no sign change between players.
            match self.transposition_table.get(&root_state.apply_move(mv)) {
                Some(&(visits, _wins)) => {
                    println!(
                        "{} visits: {}, wins: {}, outcome: {:?}",
                        mv,
                        visits,
                        _wins,
                        root_state.apply_move(mv).outcome()
                    );
                    return OrderedFloat(visits);
                }
                None => OrderedFloat(-1f32),
            }
        };
        if VERBOSE_PERF {
            // let end = PreciseTime::now();
            // println!(
            //     "{} seconds for {} rollouts",
            //     start.to(end),
            //     achieved_rollouts
            // );
            // println!(
            //     "size of the transposition table after rollouts: {}",
            //     self.transposition_table.len()
            // );
        }
        let moves = root_state.legal_moves();
        return *moves.iter().max_by_key(|&m| score(m)).unwrap();
    }
    fn reset_game_specific_state(&mut self) {
        self.transposition_table.clear();
    }
}

type TicTacMove = u8;

impl GameMove for TicTacMove {}

#[derive(Copy, Clone, Hash, Eq, PartialEq, PartialOrd, Ord)]
struct TicTacBoard {
    p0: u16,
    p1: u16,
}

impl GameBoard for TicTacBoard {
    type Move = TicTacMove;
    fn new() -> Self {
        return TicTacBoard { p0: 0u16, p1: 0u16 };
    }
    fn outcome(&self) -> GameOutcome {
        // 111
        // 111
        // 111
        static TIC_TAC_WIN_MASKS: [u16; 8] = [
            0b000_000_111u16,
            0b000_111_000u16,
            0b111_000_000u16,
            0b100_100_100u16,
            0b010_010_010u16,
            0b001_001_001u16,
            0b100_010_001u16,
            0b001_010_100u16,
        ];

        for &mask in &TIC_TAC_WIN_MASKS {
            if (mask & self.p0) == mask {
                return GameOutcome::Winner(Player::P0);
            }
        }
        for &mask in &TIC_TAC_WIN_MASKS {
            if (mask & self.p1) == mask {
                return GameOutcome::Winner(Player::P1);
            }
        }
        if (self.p0 | self.p1) == 0b0000_0001_1111_1111 {
            return GameOutcome::Draw;
        }
        return GameOutcome::Ongoing;
    }
    fn whose_turn(&self) -> Player {
        if self.p0.count_ones() > self.p1.count_ones() {
            return Player::P1;
        }
        return Player::P0;
    }
    fn apply_move(&self, m: TicTacMove) -> Self {
        match self.whose_turn() {
            Player::P0 => {
                return TicTacBoard {
                    p0: self.p0 | (1 << m),
                    p1: self.p1,
                }
            }
            Player::P1 => {
                return TicTacBoard {
                    p0: self.p0,
                    p1: self.p1 | (1 << m),
                }
            }
        }
    }
    fn legal_moves(&self) -> SomeMoves<TicTacMove> {
        let open_spots = !(self.p0 | self.p1) & 0b0000_0001_1111_1111;
        let mut result = SomeMoves::new();
        for idx in one_idxs(open_spots as u64) {
            result.push(idx);
        }
        return result;
    }
    fn afterstates(&self) -> SomeBoards<TicTacBoard> {
        let mut result = SomeBoards::new();
        for m in self.legal_moves() {
            result.push(self.apply_move(m));
        }
        return result;
    }
    fn unreachable_from(&self, state: Self) -> bool {
        return (state.p0 & self.p1 != 0) | (state.p1 & self.p0 != 0);
    }
    fn canonical(&self) -> Self {
        *self
    }
}

impl fmt::Display for TicTacBoard {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        static BOARD_STRING: [&str; 9] = ["_|", "_", "|_", "_|", "_", "|_", " |", " ", "| "];
        let mut parts = Vec::new();
        for i in 0..9 {
            if ((self.p0 >> i) & 1) == 1 {
                parts.push("X");
            } else if ((self.p1 >> i) & 1) == 1 {
                parts.push("O");
            } else {
                parts.push(BOARD_STRING[i]);
            }
            if i % 3 == 2 {
                parts.push("\n");
            }
        }
        parts.push("\n");
        return write!(f, "{}", parts[..].join(""));
    }
}

impl fmt::Debug for TicTacBoard {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        static BOARD_STRING: [&str; 9] = ["_|", "_", "|_", "_|", "_", "|_", " |", " ", "| "];
        let mut parts = Vec::new();
        for i in 0..9 {
            if ((self.p0 >> i) & 1) == 1 {
                parts.push("0");
            } else if ((self.p1 >> i) & 1) == 1 {
                parts.push("X");
            } else {
                parts.push(BOARD_STRING[i]);
            }
        }
        return write!(f, "{}", parts[..].join(""));
    }
}

type YavalathMove = u8;

type YavalathBoard = BitBoard;

impl GameBoard for YavalathBoard {
    type Move = YavalathMove;
    fn new() -> Self {
        return BitBoard::empty();
    }
    fn outcome(&self) -> GameOutcome {
        match self.outcome {
            None => check_game_outcome(*self).0,
            Some(outcome) => outcome,
        }
    }
    fn whose_turn(&self) -> Player {
        let n_p0_moves = self.p0.bits.count_ones();
        let n_p1_moves = self.p1.bits.count_ones();
        if n_p0_moves <= n_p1_moves {
            return Player::P0;
        } else {
            return Player::P1;
        }
    }
    fn apply_move(&self, m: Self::Move) -> Self {
        let result;
        match self.whose_turn() {
            Player::P0 => {
                result = Self {
                    p0: PlayerBitBoard {
                        bits: self.p0.bits | (1 << m),
                    },
                    p1: self.p1,
                    outcome: Some(check_game_outcome_after_move(*self, m)),
                };
            }
            Player::P1 => {
                result = Self {
                    p0: self.p0,
                    p1: PlayerBitBoard {
                        bits: self.p1.bits | (1 << m),
                    },
                    outcome: Some(check_game_outcome_after_move(*self, m)),
                };
            }
        }
        return result.canonical();
    }
    fn legal_moves(&self) -> SomeMoves<Self::Move> {
        let mut result = SomeMoves::new();
        let free_positions = !(self.p0.bits | self.p1.bits) & EXCLUDE_TOP_BITS;
        for i in one_idxs(free_positions) {
            result.push(i)
        }
        return result;
    }
    fn afterstates(&self) -> SomeBoards<Self> {
        let mut result = SomeBoards::new();
        let free_positions = !(self.p0.bits | self.p1.bits) & EXCLUDE_TOP_BITS;
        for i in one_idxs(free_positions) {
            result.push(self.apply_move(i))
        }
        let occupied = 61 - free_positions.count_ones();
        if occupied < CANON_LIMIT {
            quicksort(&mut result[..]);
            result.dedup();
        }
        return result;
    }
    fn unreachable_from(&self, state: Self) -> bool {
        if (self.p0.bits & state.p1.bits) != 0 {
            return false;
        }
        if (self.p1.bits & state.p0.bits) != 0 {
            return false;
        }
        return true;
    }
    fn canonical(&self) -> Self {
        return canonical(*self);
    }
}

#[derive(Copy, Clone, Hash, Eq, PartialEq, PartialOrd, Ord)]
struct PlayerBitBoard {
    bits: u64,
}

#[derive(Copy, Clone, Hash, Eq, PartialEq, PartialOrd, Ord)]
struct BitBoard {
    p0: PlayerBitBoard,
    p1: PlayerBitBoard,
    outcome: Option<GameOutcome>,
}

type TTable = HashMap<BitBoard, (f32, f32)>;
type SomeBoards3 = SmallVec<[BitBoard; 32]>;

impl BitBoard {
    fn empty() -> BitBoard {
        return BitBoard {
            p0: PlayerBitBoard { bits: 0 },
            p1: PlayerBitBoard { bits: 0 },
            outcome: Some(GameOutcome::Ongoing),
        };
    }
    fn make(p0: u64, p1: u64) -> Self {
        return BitBoard {
            p0: PlayerBitBoard { bits: p0 },
            p1: PlayerBitBoard { bits: p1 },
            outcome: None,
        };
    }
    fn full(&self) -> bool {
        return !(self.p0.bits | self.p1.bits) & EXCLUDE_TOP_BITS == 0;
    }
}

impl fmt::Display for BitBoard {
    // This trait requires `fmt` with this exact signature.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // Write strictly the first element into the supplied output
        // stream: `f`. Returns `fmt::Result` which indicates whether the
        // operation succeeded or failed. Note that `write!` uses syntax which
        // is very similar to `println!`.
        write!(f, "{:016X}{:016X}", self.p0.bits, self.p1.bits)
    }
}

impl fmt::Debug for BitBoard {
    // This trait requires `fmt` with this exact signature.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // Write strictly the first element into the supplied output
        // stream: `f`. Returns `fmt::Result` which indicates whether the
        // operation succeeded or failed. Note that `write!` uses syntax which
        // is very similar to `println!`.
        write!(f, "{:016X}{:016X}", self.p0.bits, self.p1.bits)
    }
}

fn yavalath_rotate(bb: u64) -> u64 {
    static ROLLED_RING_POSITIONS: u64 = 0x2082082082082082;

    let shifted = bb << 1;
    let checked_rolled_bits = shifted & ROLLED_RING_POSITIONS;
    let with_cleared_rolled_bits = shifted & (!ROLLED_RING_POSITIONS);
    let with_set_rolled_bits = with_cleared_rolled_bits | (checked_rolled_bits >> 6);

    // There is a middle ring, of size 1. Check that one
    let including_center = with_set_rolled_bits | (bb & 1);

    // println!(" {:064b} bb", bb);
    // println!(" {:064b} shifted", shifted);
    // println!(" {:064b} check_rolled_bits", checked_rolled_bits);
    // println!(" {:064b} with_cleared_rolled_bits", with_cleared_rolled_bits);
    // println!(" {:064b} wit set rolled bits ", with_set_rolled_bits);
    // println!("");
    return including_center;
}

fn rotate_board(bb: BitBoard) -> BitBoard {
    return BitBoard {
        p0: PlayerBitBoard {
            bits: yavalath_rotate(bb.p0.bits),
        },
        p1: PlayerBitBoard {
            bits: yavalath_rotate(bb.p1.bits),
        },
        outcome: bb.outcome,
    };
}

fn mirror_board(bb: BitBoard) -> BitBoard {
    return BitBoard {
        p0: PlayerBitBoard {
            bits: yavalath_mirror(bb.p0.bits),
        },
        p1: PlayerBitBoard {
            bits: yavalath_mirror(bb.p1.bits),
        },
        outcome: bb.outcome,
    };
}

fn yavalath_mirror(n: u64) -> u64 {
    let mut result = n;
    for (p1, p2) in &constants::MIRROR_IDXS {
        let p1_set = (n >> p1) & 1;
        let p2_set = (n >> p2) & 1;
        let xor = p1_set ^ p2_set;
        let both_xor = (xor << p1) | (xor << p2);
        result ^= both_xor
    }
    return result;
}

fn canonical(bb: BitBoard) -> BitBoard {
    if (bb.p0.bits | bb.p1.bits).count_ones() >= CANON_LIMIT {
        return bb;
    }
    let mut bbs = SmallVec::<[BitBoard; 12]>::new();
    bbs.push(bb);
    for _ in 0..5 {
        let prev = *bbs.last().unwrap();
        bbs.push(rotate_board(prev));
    }
    bbs.push(mirror_board(bb));
    for _ in 0..5 {
        let prev = *bbs.last().unwrap();
        bbs.push(rotate_board(prev));
    }

    return *bbs.iter().min().unwrap();
}

fn any_mask(bb: u64, masks: &[u64]) -> (bool, Option<u64>) {
    // Faster to reverse the loops, but this is not in
    // the search procedure, and having mask is convenient.
    for &mask in masks {
        let mut r_mask = mask;
        for _ in 0..3 {
            if (r_mask & bb) == r_mask {
                return (true, Some(r_mask));
            };
            r_mask = yavalath_rotate(r_mask);
        }
    }
    return (false, None);
}

fn check_game_outcome(bb: BitBoard) -> (GameOutcome, Option<u64>) {
    let outcome;
    let mut mask = None;
    if bb.full() {
        // Draw
        outcome = GameOutcome::Draw;
    } else if any_mask(bb.p0.bits, &constants::WINNING_MASKS).0 {
        // PO Wins
        outcome = GameOutcome::Winner(Player::P0);
        mask = any_mask(bb.p0.bits, &constants::WINNING_MASKS).1
    } else if any_mask(bb.p1.bits, &constants::WINNING_MASKS).0 {
        // P1 Wins
        outcome = GameOutcome::Winner(Player::P0);
        mask = any_mask(bb.p1.bits, &constants::WINNING_MASKS).1
    } else if any_mask(bb.p0.bits, &constants::LOSING_MASKS).0 {
        // P0 Loses
        outcome = GameOutcome::Winner(Player::P0);
        mask = any_mask(bb.p0.bits, &constants::LOSING_MASKS).1
    } else if any_mask(bb.p1.bits, &constants::LOSING_MASKS).0 {
        // P1 Loses
        outcome = GameOutcome::Winner(Player::P0);
        mask = any_mask(bb.p1.bits, &constants::LOSING_MASKS).1
    } else {
        // Ongoing
        outcome = GameOutcome::Ongoing;
    }
    return (outcome, mask);
}

fn other_player(player: Player) -> Player {
    match player {
        Player::P0 => Player::P1,
        Player::P1 => Player::P0,
    }
}

fn check_game_outcome_after_move(bb: YavalathBoard, m: YavalathMove) -> GameOutcome {
    use GameOutcome::*;
    use Player::*;
    let player = bb.whose_turn();
    let player_bits;
    let occupied;
    match player {
        P0 => {
            player_bits = bb.p0.bits | (1 << m);
            occupied = player_bits | bb.p1.bits;
        }
        P1 => {
            player_bits = bb.p1.bits | (1 << m);
            occupied = player_bits | bb.p0.bits;
        }
    }
    if occupied == EXCLUDE_TOP_BITS {
        return Draw;
    }
    for &mask in &constants::WIN_CHECKS[m as usize] {
        if (mask & player_bits) == mask {
            return Winner(player);
        }
    }
    for &mask in &constants::LOSE_CHECKS[m as usize] {
        if (mask & player_bits) == mask {
            return Winner(other_player(player));
        }
    }
    return GameOutcome::Ongoing;
}

fn parse_board_hex(s: String) -> Result<BitBoard, std::num::ParseIntError> {
    let p0bb = u64::from_str_radix(&s[..16], 16)?;
    let p1bb = u64::from_str_radix(&s[16..], 16)?;
    if p0bb & p1bb != 0 {
        panic!("Invalid Board!");
    }
    return Ok(BitBoard::make(p0bb, p1bb));
}

fn random_bit_from_mask(mut mask: u64, randomness: u8) -> u64 {
    let k = mask.count_ones();
    let i = (randomness as u32) % k;
    for _ in 0..i {
        mask &= mask - 1;
    }
    return 1 << mask.trailing_zeros();
}

// fn random_move(bb: BitBoard, randomness: u8) -> BitBoard {
//     let free_positions = !(bb.p0.bits | bb.p1.bits) & EXCLUDE_TOP_BITS;
//     let m = random_bit_from_mask(free_positions, randomness);
//     if n_moves_played(bb) & 1 == 0 {
//         // p0s move
//         return BitBoard {
//             p0: PlayerBitBoard {
//                 bits: bb.p0.bits | m,
//             },
//             p1: bb.p1,
//         };
//     } else {
//         // p1s move
//         return BitBoard {
//             p0: bb.p0,
//             p1: PlayerBitBoard {
//                 bits: bb.p1.bits | m,
//             },
//         };
//     }
// }

fn one_idxs(n: u64) -> SomeIdxs {
    let mut result = SomeIdxs::new();
    for i in 0..64 {
        if (n & (1 << i)) != 0 {
            result.push(i as u8);
        }
    }
    return result;
}

fn n_moves_played(bb: BitBoard) -> u32 {
    return (bb.p0.bits | bb.p1.bits).count_ones();
}

fn uniform_random_choice<Item>(xs: &mut dyn ExactSizeIterator<Item = Item>) -> Option<Item> {
    let mut rng = thread_rng();
    let n = xs.len();
    if n == 0 {
        panic!("Uniform Random Choice from empte iterator?");
    }
    return xs.nth(rng.gen_range(0, n));
}

fn random_choice(scores: SmallVec<[f32; 64]>) -> u8 {
    let total: f32 = scores.iter().sum();
    let n = scores.len() as f32;
    let vs = scores
        .iter()
        .map(|score| (n * score) / total)
        .scan(0.0, |state, x| {
            *state += x;
            Some(*state)
        })
        .collect::<SmallVec<[f32; 64]>>();

    let r = thread_rng().gen_range(0.0, n);
    for i in 0..scores.len() {
        if r < vs[i] {
            return i as u8;
        }
    }
    panic!();
}

fn compute_tables(some_masks: &[u64], n: usize) {
    let mut all_masks = Vec::new();
    for &mask in some_masks.iter() {
        all_masks.push(mask);
        all_masks.push(yavalath_rotate(mask));
        all_masks.push(yavalath_rotate(yavalath_rotate(mask)));
    }
    let mut all_rows = Vec::new();
    for i in 0..61 {
        let mut relevant = Vec::new();
        for &mask in all_masks.iter() {
            if mask & (1 << i) != 0 {
                relevant.push(mask);
            }
        }
        while relevant.len() < n {
            relevant.push(0);
        }
        all_rows.push(relevant);
    }
    println!("{:?}", all_rows);
}

pub extern "C" fn ai_pick_move(board_hex: String, terminate: &mut (FnMut() -> bool)) -> i32 {
    match parse_board_hex(board_hex) {
        Ok(bb) => {
            let mut player = MCTS::<YavalathBoard>::new(100000);
            let ai_move = player.choose_move(bb, terminate);
            ai_move as i32
        },
        Err(_) => -1,
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CompleteOutcome {
    outcome: GameOutcome,
    locations: Option<[i32; 4]>,
}

fn check_game_string_worker(board_hex: String) -> CompleteOutcome {
    match parse_board_hex(board_hex) {
        Ok(bb) => {
            let (outcome, mask_opt) = check_game_outcome(bb);
            match outcome {
                GameOutcome::Winner(player) => {
                    let mut locations: [i32; 4] = [-1, -1, -1, -1];
                    let mut i = 0;
                    match mask_opt {
                        Some(mask) => {
                            for idx in one_idxs(mask) {
                                locations[i] = idx as i32;
                                i += 1;
                            }
                            CompleteOutcome {
                                outcome: GameOutcome::Winner(player),
                                locations: Some(locations),
                            }
                        }
                        None => CompleteOutcome {
                            outcome: GameOutcome::Winner(player),
                            locations: None,
                        },
                    }
                }
                _ => CompleteOutcome {
                    outcome: outcome,
                    locations: None,
                },
            }
        }
        Err(_e) => CompleteOutcome {
            outcome: GameOutcome::Invalid,
            locations: None,
        },
    }
}

pub extern "C" fn check_game_string(board_hex: String) -> String {
    serde_json::to_string(&check_game_string_worker(board_hex)).unwrap()
}
