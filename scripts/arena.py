#!/usr/bin/env python3
import argparse
import itertools
import json
import math
import os
import random
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Dict, List, Optional, Tuple

ROOT = Path(__file__).resolve().parents[1]
WORKTREE_ROOT = ROOT / ".arena" / "worktrees"

BOARD_RADIUS = 4
BOARD_CELLS = 61
SWAP_MOVE = BOARD_CELLS
LINE_DIRS = [(1, 0), (0, 1), (1, -1)]


def axial_cells() -> List[Tuple[int, int]]:
    cells = []
    for q in range(-BOARD_RADIUS, BOARD_RADIUS + 1):
        r_min = max(-BOARD_RADIUS, -q - BOARD_RADIUS)
        r_max = min(BOARD_RADIUS, -q + BOARD_RADIUS)
        for r in range(r_min, r_max + 1):
            cells.append((q, r))
    return cells


CELLS = axial_cells()
CELL_TO_IDX = {c: i for i, c in enumerate(CELLS)}


def all_lines(k: int) -> List[Tuple[int, ...]]:
    lines = set()
    for q, r in CELLS:
        for dq, dr in LINE_DIRS:
            run = []
            for step in range(k):
                nxt = (q + dq * step, r + dr * step)
                if nxt not in CELL_TO_IDX:
                    run = []
                    break
                run.append(CELL_TO_IDX[nxt])
            if len(run) == k:
                lines.add(tuple(run))
    return sorted(lines)


THREE_LINES = all_lines(3)
FOUR_LINES = all_lines(4)


@dataclass
class Position:
    p0: int = 0
    p1: int = 0
    turn: int = 0
    ply: int = 0

    def occupied(self) -> int:
        return self.p0 | self.p1

    def can_swap(self) -> bool:
        return self.ply == 1 and self.turn == 1

    def legal(self) -> List[int]:
        moves = [i for i in range(BOARD_CELLS) if ((self.occupied() >> i) & 1) == 0]
        if self.can_swap():
            moves.append(SWAP_MOVE)
        return moves

    def apply(self, mv: int) -> Optional[Tuple["Position", Optional[Tuple[int, int]]]]:
        if mv == SWAP_MOVE:
            if not self.can_swap():
                return None
            return Position(self.p1, self.p0, self.turn ^ 1, self.ply), None
        if mv < 0 or mv >= BOARD_CELLS:
            return None
        if (self.occupied() >> mv) & 1:
            return None
        nxt = Position(self.p0, self.p1, self.turn ^ 1, self.ply + 1)
        if self.turn == 0:
            nxt.p0 |= 1 << mv
        else:
            nxt.p1 |= 1 << mv
        return nxt, (self.turn, mv)


def has_line(bits: int, lines: List[Tuple[int, ...]]) -> Optional[Tuple[int, ...]]:
    for line in lines:
        if all(((bits >> i) & 1) for i in line):
            return line
    return None


def outcome(pos: Position, just_played: Optional[Tuple[int, int]]) -> Tuple[str, Optional[int]]:
    if just_played is not None:
        player, _ = just_played
        bits = pos.p0 if player == 0 else pos.p1
        if has_line(bits, FOUR_LINES):
            return "win", player
        if has_line(bits, THREE_LINES):
            return "lose", player
    else:
        for player, bits in ((0, pos.p0), (1, pos.p1)):
            if has_line(bits, FOUR_LINES):
                return "win", player
            if has_line(bits, THREE_LINES):
                return "lose", player
    if pos.ply >= BOARD_CELLS:
        return "draw", None
    return "ongoing", None


def encode_board_hex(pos: Position) -> str:
    return f"{((pos.p0 << 64) | pos.p1):032x}"


class AgentProc:
    def __init__(self, cmd: List[str]):
        self.proc = subprocess.Popen(
            cmd,
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            bufsize=1,
        )

    def pick(self, board_hex: str) -> int:
        assert self.proc.stdin and self.proc.stdout
        self.proc.stdin.write(board_hex + "\n")
        self.proc.stdin.flush()
        out = self.proc.stdout.readline().strip()
        return int(out)

    def close(self):
        if self.proc.poll() is None:
            self.proc.terminate()
            try:
                self.proc.wait(timeout=2)
            except subprocess.TimeoutExpired:
                self.proc.kill()


def ensure_worktree(commit: str) -> Path:
    WORKTREE_ROOT.mkdir(parents=True, exist_ok=True)
    short = subprocess.check_output(["git", "rev-parse", "--short", commit], cwd=ROOT, text=True).strip()
    wt = WORKTREE_ROOT / short
    if not wt.exists():
        subprocess.check_call(["git", "worktree", "add", "--detach", str(wt), commit], cwd=ROOT)
    return wt


_BIN_CACHE: Dict[str, Path] = {}

def ensure_local_binary() -> Path:
    subprocess.check_call(["cargo", "build", "--release", "--bin", "arena_agent"], cwd=ROOT / "crate")
    return ROOT / "crate" / "target" / "release" / "arena_agent"

def ensure_agent_binary(commit: str) -> Path:
    if commit in _BIN_CACHE:
        return _BIN_CACHE[commit]
    if commit in ("HEAD", "WORKTREE", "."):
        bin_path = ensure_local_binary()
        _BIN_CACHE[commit] = bin_path
        return bin_path
    wt = ensure_worktree(commit)
    subprocess.check_call(["cargo", "build", "--release", "--bin", "arena_agent"], cwd=wt / "crate")
    bin_path = wt / "crate" / "target" / "release" / "arena_agent"
    _BIN_CACHE[commit] = bin_path
    return bin_path


def command_for_player(player: Dict, game_seed: int) -> List[str]:
    commit = player.get("commit", "HEAD")
    binary = ensure_agent_binary(commit)
    strategy = player["strategy"]
    cmd = [str(binary), "--strategy", strategy, "--seed", str(game_seed)]
    if strategy == "strength":
        cmd += ["--strength", str(player.get("strength", 2)), "--time-ms", str(player.get("time_ms", 25.0))]
    return cmd


def play_game(p0: Dict, p1: Dict, seed: int) -> float:
    agents = [AgentProc(command_for_player(p0, seed * 2 + 1)), AgentProc(command_for_player(p1, seed * 2 + 2))]
    pos = Position()
    jp = None
    winner = None
    try:
        while True:
            state, player = outcome(pos, jp)
            if state == "win":
                winner = player
                break
            if state == "lose":
                winner = player ^ 1
                break
            if state == "draw":
                return 0.5

            mv = agents[pos.turn].pick(encode_board_hex(pos))
            applied = pos.apply(mv)
            if applied is None:
                winner = pos.turn ^ 1
                break
            pos, jp = applied

        return 1.0 if winner == 0 else 0.0
    finally:
        for a in agents:
            a.close()


def fit_elo(players: List[str], games: List[Tuple[int, int, float]], iters: int = 400) -> List[float]:
    n = len(players)
    r = [0.0] * n
    k = math.log(10) / 400.0
    lr = 0.6
    for _ in range(iters):
        grad = [0.0] * n
        for i, j, s in games:
            p = 1.0 / (1.0 + math.exp(-k * (r[i] - r[j])))
            d = s - p
            grad[i] += d
            grad[j] -= d
        mean = sum(grad) / n
        for idx in range(n):
            r[idx] += lr * (grad[idx] - mean)
        m = sum(r) / n
        r = [x - m for x in r]
    return r


def bootstrap_ci(players: List[str], games: List[Tuple[int, int, float]], samples: int, seed: int):
    rng = random.Random(seed)
    all_ratings = [[] for _ in players]
    for _ in range(samples):
        sample = [games[rng.randrange(len(games))] for _ in range(len(games))]
        rs = fit_elo(players, sample)
        for i, v in enumerate(rs):
            all_ratings[i].append(v)
    ci = []
    for arr in all_ratings:
        arr.sort()
        lo = arr[int(0.025 * len(arr))]
        hi = arr[int(0.975 * len(arr))]
        ci.append((lo, hi))
    return ci


def run_tournament(config: Dict):
    players = config["players"]
    names = [p["name"] for p in players]
    idx = {n: i for i, n in enumerate(names)}
    games: List[Tuple[int, int, float]] = []

    rounds = config.get("games_per_pair", 40)
    seed = config.get("seed", 1)
    random.seed(seed)

    for a, b in itertools.combinations(players, 2):
        ia, ib = idx[a["name"]], idx[b["name"]]
        for g in range(rounds):
            s = seed + g + ia * 10_000 + ib * 100
            first_a = (g % 2 == 0)
            if first_a:
                score = play_game(a, b, s)
                games.append((ia, ib, score))
            else:
                score_b = play_game(b, a, s)
                games.append((ia, ib, 1.0 - score_b))

    ratings = fit_elo(names, games)
    ci = bootstrap_ci(names, games, config.get("bootstrap_samples", 200), seed + 999)

    table = []
    for i, name in enumerate(names):
        table.append(
            {
                "name": name,
                "elo": round(ratings[i], 1),
                "ci95": [round(ci[i][0], 1), round(ci[i][1], 1)],
            }
        )
    table.sort(key=lambda x: x["elo"], reverse=True)

    print(json.dumps({"games": len(games), "ratings": table}, indent=2))


def main():
    parser = argparse.ArgumentParser(description="Cross-commit Yavalath arena")
    parser.add_argument("--config", required=True, help="JSON config path")
    args = parser.parse_args()
    cfg = json.loads(Path(args.config).read_text())
    run_tournament(cfg)


if __name__ == "__main__":
    main()
