use wasm_bindgen::prelude::*;

mod engine;

#[wasm_bindgen]
pub fn check_game_outcome(board_hex: &str) -> String {
    let Ok(pos) = engine::parse_board_hex(board_hex) else {
        return engine::encode_outcome(engine::Outcome::Invalid);
    };

    let just_played = if pos.ply == 0 { None } else { Some(pos.turn ^ 1) };
    engine::encode_outcome(engine::outcome(pos, just_played))
}

#[wasm_bindgen]
pub fn pick_move(board_hex: &str, thinking_time_ms: f64) -> i32 {
    let Ok(pos) = engine::parse_board_hex(board_hex) else {
        return -1;
    };

    engine::best_move(pos, thinking_time_ms)
        .map(i32::from)
        .unwrap_or(-1)
}
