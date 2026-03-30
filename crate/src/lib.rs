use wasm_bindgen::prelude::*;

mod engine;

#[wasm_bindgen]
pub fn check_game_outcome(board_hex: &str) -> String {
    let Ok(pos) = engine::parse_board_hex(board_hex) else {
        return engine::encode_outcome(engine::Outcome::Invalid);
    };

    engine::encode_outcome(engine::outcome(pos, None))
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

#[wasm_bindgen]
pub fn pick_move_with_strength(board_hex: &str, thinking_time_ms: f64, strength: u8) -> i32 {
    let Ok(pos) = engine::parse_board_hex(board_hex) else {
        return -1;
    };

    engine::best_move_with_strength(pos, thinking_time_ms, strength)
        .map(i32::from)
        .unwrap_or(-1)
}
