import './style.css';
import initWasm, { check_game_outcome, pick_move_with_strength } from './wasm/yavalath_engine.js';

const RADIUS = 4;
const BOARD_CELLS = 61;
const SWAP_MOVE = BOARD_CELLS;
const HEX_SIZE = 42;
const W = Math.sqrt(3) * HEX_SIZE;
const H = 2 * HEX_SIZE;
const X_OFFSET = 450;
const Y_OFFSET = 380;

const boardSvg = document.getElementById('board');
const statusEl = document.getElementById('status');
const newGameBtn = document.getElementById('new-game');
const aiMoveBtn = document.getElementById('ai-move');
const undoMoveBtn = document.getElementById('undo-move');
const swapMoveBtn = document.getElementById('swap-move');
const aiFirstMoveBtn = document.getElementById('ai-first-move');
const aiStrengthSelect = document.getElementById('ai-strength');

const AI_PRESETS = {
  0: { strength: 0, budgetMs: 140 },
  1: { strength: 1, budgetMs: 320 },
  2: { strength: 2, budgetMs: 900 },
  3: { strength: 3, budgetMs: 1800 },
};

const state = {
  p0: Array(BOARD_CELLS).fill(false),
  p1: Array(BOARD_CELLS).fill(false),
  turn: 0,
  ply: 0,
  gameOver: false,
  aiPlayer: 1,
  busy: false,
  line: [],
  history: [],
  aiStrength: Number(aiStrengthSelect?.value ?? 2),
};

const coords = [];
for (let q = -RADIUS; q <= RADIUS; q += 1) {
  const rMin = Math.max(-RADIUS, -q - RADIUS);
  const rMax = Math.min(RADIUS, -q + RADIUS);
  for (let r = rMin; r <= rMax; r += 1) {
    coords.push([q, r]);
  }
}

const elements = coords.map((coord, idx) => createHex(coord, idx));

function cloneSnapshot() {
  return {
    p0: [...state.p0],
    p1: [...state.p1],
    turn: state.turn,
    ply: state.ply,
    gameOver: state.gameOver,
    line: [...state.line],
  };
}

function restoreSnapshot(snapshot) {
  state.p0 = [...snapshot.p0];
  state.p1 = [...snapshot.p1];
  state.turn = snapshot.turn;
  state.ply = snapshot.ply;
  state.gameOver = snapshot.gameOver;
  state.line = [...snapshot.line];
}

function occupied(idx) {
  return state.p0[idx] || state.p1[idx];
}

function canSwap() {
  return state.ply === 1 && state.turn === 1;
}

function createHex([q, r], idx) {
  const [cx, cy] = axialToPixel(q, r);
  const points = [];
  for (let i = 0; i < 6; i += 1) {
    const angle = (Math.PI / 3) * i + Math.PI / 6;
    points.push(`${cx + HEX_SIZE * Math.cos(angle)},${cy + HEX_SIZE * Math.sin(angle)}`);
  }
  const poly = document.createElementNS('http://www.w3.org/2000/svg', 'polygon');
  poly.setAttribute('points', points.join(' '));
  poly.classList.add('cell');
  poly.dataset.idx = String(idx);
  poly.addEventListener('click', onCellClick);
  boardSvg.appendChild(poly);
  return poly;
}

function axialToPixel(q, r) {
  return [X_OFFSET + W * (q + r / 2), Y_OFFSET + H * 0.75 * r];
}

function boardHex() {
  const bitsP0 = Array(64).fill('0');
  const bitsP1 = Array(64).fill('0');
  state.p0.forEach((filled, idx) => {
    if (filled) bitsP0[idx] = '1';
  });
  state.p1.forEach((filled, idx) => {
    if (filled) bitsP1[idx] = '1';
  });

  const bits = bitsP0.slice().reverse().join('') + bitsP1.slice().reverse().join('');
  let out = '';
  for (let i = 0; i < bits.length; i += 4) {
    out += Number.parseInt(bits.slice(i, i + 4), 2).toString(16);
  }
  return out;
}

function readOutcome() {
  return JSON.parse(check_game_outcome(boardHex()));
}

function applyMove(idx) {
  if (state.gameOver || state.busy || occupied(idx)) return false;
  state.history.push(cloneSnapshot());
  if (state.turn === 0) state.p0[idx] = true;
  else state.p1[idx] = true;
  state.turn ^= 1;
  state.ply += 1;
  return true;
}

function applySwap() {
  if (state.gameOver || state.busy || !canSwap()) return false;
  state.history.push(cloneSnapshot());
  const oldP0 = state.p0;
  state.p0 = state.p1;
  state.p1 = oldP0;
  state.turn ^= 1;
  return true;
}

function undoOneMove() {
  if (state.busy) return;
  const prev = state.history.pop();
  if (!prev) return;
  restoreSnapshot(prev);
  refresh();
}

function onCellClick(e) {
  const idx = Number(e.currentTarget.dataset.idx);
  if (!applyMove(idx)) return;
  refresh();
  maybeRunAi();
}

function refresh() {
  elements.forEach((el, idx) => {
    el.classList.remove('p0', 'p1', 'line');
    if (state.p0[idx]) el.classList.add('p0');
    if (state.p1[idx]) el.classList.add('p1');
  });

  const outcome = readOutcome();
  state.gameOver = outcome.state === 'win' || outcome.state === 'lose' || outcome.state === 'draw';
  state.line = outcome.line || [];
  state.line.forEach((i) => elements[i]?.classList.add('line'));

  if (outcome.state === 'ongoing') {
    if (state.busy) {
      statusEl.textContent = 'AI is thinking...';
    } else if (canSwap()) {
      statusEl.textContent = 'Blue may play a move or use swap rule.';
    } else {
      statusEl.textContent = `Turn: ${state.turn === 0 ? 'Red' : 'Blue'}`;
    }
  } else if (outcome.state === 'draw') {
    statusEl.textContent = 'Draw.';
  } else if (outcome.state === 'win') {
    statusEl.textContent = `${outcome.winner === 0 ? 'Red' : 'Blue'} wins (4 in a row).`;
  } else if (outcome.state === 'lose') {
    statusEl.textContent = `${outcome.loser === 0 ? 'Red' : 'Blue'} loses (made 3 in a row).`;
  } else {
    statusEl.textContent = 'Invalid board state.';
  }

  swapMoveBtn.disabled = !canSwap() || state.busy || state.gameOver;
  undoMoveBtn.disabled = state.history.length === 0 || state.busy;
}

async function maybeRunAi() {
  if (state.gameOver || state.busy) return;
  if (state.turn !== state.aiPlayer) return;

  state.busy = true;
  refresh();
  await new Promise((resolve) => setTimeout(resolve, 10));

  const preset = AI_PRESETS[state.aiStrength] ?? AI_PRESETS[2];
  const mv = pick_move_with_strength(boardHex(), preset.budgetMs, preset.strength);
  if (mv === SWAP_MOVE) {
    applySwap();
  } else if (mv >= 0 && mv < BOARD_CELLS) {
    applyMove(mv);
  }

  state.busy = false;
  refresh();
}

function resetGame() {
  state.p0 = Array(BOARD_CELLS).fill(false);
  state.p1 = Array(BOARD_CELLS).fill(false);
  state.turn = 0;
  state.ply = 0;
  state.gameOver = false;
  state.busy = false;
  state.line = [];
  state.history = [];
}

newGameBtn.addEventListener('click', () => {
  state.aiPlayer = 1;
  resetGame();
  refresh();
});

undoMoveBtn.addEventListener('click', () => {
  undoOneMove();
});

swapMoveBtn.addEventListener('click', () => {
  if (!applySwap()) return;
  refresh();
  maybeRunAi();
});

aiMoveBtn.addEventListener('click', () => {
  maybeRunAi();
});

aiFirstMoveBtn.addEventListener('click', () => {
  if (state.ply !== 0 || state.busy) return;
  state.aiPlayer = 0;
  maybeRunAi();
});

aiStrengthSelect?.addEventListener('change', () => {
  state.aiStrength = Number(aiStrengthSelect.value);
});

await initWasm();
refresh();
