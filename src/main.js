import './style.css';
import initWasm, { check_game_outcome, pick_move } from './wasm/yavalath_engine.js';

const RADIUS = 4;
const HEX_SIZE = 42;
const W = Math.sqrt(3) * HEX_SIZE;
const H = 2 * HEX_SIZE;
const X_OFFSET = 450;
const Y_OFFSET = 380;

const boardSvg = document.getElementById('board');
const statusEl = document.getElementById('status');
const newGameBtn = document.getElementById('new-game');
const aiMoveBtn = document.getElementById('ai-move');

const state = {
  moves: [],
  gameOver: false,
  aiPlayer: 1,
  busy: false,
  line: [],
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

function createHex([q, r], idx) {
  const [cx, cy] = axialToPixel(q, r);
  const points = [];
  for (let i = 0; i < 6; i += 1) {
    const angle = Math.PI / 3 * i + Math.PI / 6;
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
  return [
    X_OFFSET + W * (q + r / 2),
    Y_OFFSET + (H * 0.75) * r,
  ];
}

function onCellClick(e) {
  if (state.gameOver || state.busy) return;
  const idx = Number(e.currentTarget.dataset.idx);
  if (state.moves.includes(idx)) return;
  state.moves.push(idx);
  refresh();
  maybeRunAi();
}

function gameStateToBoardHex(moves) {
  const bitsP0 = Array(64).fill('0');
  const bitsP1 = Array(64).fill('0');
  moves.forEach((move, i) => {
    if (i % 2 === 0) bitsP0[move] = '1';
    else bitsP1[move] = '1';
  });
  const bits = bitsP0.slice().reverse().join('') + bitsP1.slice().reverse().join('');
  let out = '';
  for (let i = 0; i < bits.length; i += 4) {
    out += Number.parseInt(bits.slice(i, i + 4), 2).toString(16);
  }
  return out;
}

function readOutcome() {
  const hex = gameStateToBoardHex(state.moves);
  return JSON.parse(check_game_outcome(hex));
}

function refresh() {
  elements.forEach((el, idx) => {
    el.classList.remove('p0', 'p1', 'line');
    const turn = state.moves.indexOf(idx);
    if (turn >= 0) {
      el.classList.add(turn % 2 === 0 ? 'p0' : 'p1');
    }
  });

  const outcome = readOutcome();
  state.gameOver = outcome.state === 'win' || outcome.state === 'lose' || outcome.state === 'draw';
  state.line = outcome.line || [];
  state.line.forEach((i) => elements[i]?.classList.add('line'));

  if (outcome.state === 'ongoing') {
    statusEl.textContent = state.busy
      ? 'AI is thinking...'
      : `Turn: ${state.moves.length % 2 === 0 ? 'Red' : 'Blue'}`;
  } else if (outcome.state === 'draw') {
    statusEl.textContent = 'Draw.';
  } else if (outcome.state === 'win') {
    statusEl.textContent = `${outcome.winner === 0 ? 'Red' : 'Blue'} wins (4 in a row).`;
  } else if (outcome.state === 'lose') {
    statusEl.textContent = `${outcome.loser === 0 ? 'Red' : 'Blue'} loses (made 3 in a row).`;
  } else {
    statusEl.textContent = 'Invalid board state.';
  }
}

async function maybeRunAi() {
  if (state.gameOver || state.busy) return;
  if (state.moves.length % 2 !== state.aiPlayer) return;
  state.busy = true;
  refresh();
  await new Promise((resolve) => setTimeout(resolve, 10));
  const hex = gameStateToBoardHex(state.moves);
  const mv = pick_move(hex, 900);
  if (mv >= 0 && !state.moves.includes(mv)) {
    state.moves.push(mv);
  }
  state.busy = false;
  refresh();
}

newGameBtn.addEventListener('click', () => {
  state.moves = [];
  state.gameOver = false;
  state.busy = false;
  state.line = [];
  refresh();
});

aiMoveBtn.addEventListener('click', () => {
  maybeRunAi();
});

await initWasm();
refresh();
