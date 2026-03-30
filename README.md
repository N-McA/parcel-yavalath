# Yavalath (modernized)

This repo now ships a working browser build with:

- A Rust bitboard Yavalath engine compiled to WebAssembly via `wasm-bindgen` (no `wasm-pack` required).
- A modern Vite front-end with a simple playable SVG board.
- A stronger AI based on iterative deepening + alpha-beta search with tactical move ordering.

## How it works

### Engine

The engine (`crate/src/engine.rs`) models the 61-cell hex board as two `u64` bitboards (`p0`, `p1`).
It evaluates Yavalath rules exactly:

- make 4 in a row -> win immediately
- otherwise, make 3 in a row -> immediate loss

Search uses:

- iterative deepening over a time budget,
- negamax alpha-beta pruning,
- tactical move ordering (immediate wins/losses + center bias),
- a shape-based heuristic over all 4-cell lines.

### WASM boundary

`crate/src/lib.rs` exports:

- `check_game_outcome(boardHex) -> JSON string`
- `pick_move(boardHex, thinkingTimeMs) -> move index`

The board format is the historical 128-bit packed hex string.

### Front-end

`src/main.js` renders a 61-cell SVG board and keeps game history in JS.
It calls wasm for:

- legal outcome checks after each move,
- AI move selection.

## Run

```bash
npm install
npm run dev
```

## Build

```bash
npm run build
```

`npm run build:wasm` will:

1. ensure `wasm32-unknown-unknown` target exists,
2. install `wasm-bindgen-cli` automatically if missing,
3. compile Rust wasm,
4. generate JS bindings in `src/wasm/`.
