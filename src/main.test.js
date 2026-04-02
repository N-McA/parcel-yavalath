/** @vitest-environment jsdom */
import { describe, it, expect, beforeEach, vi } from 'vitest';

vi.mock('../src/wasm/yavalath_engine.js', () => ({
  default: vi.fn(async () => ({})),
  check_game_outcome: vi.fn(() => JSON.stringify({ state: 'ongoing', line: [] })),
  pick_move_with_strength: vi.fn(() => 1),
}));

describe('AI turn flow', () => {
  beforeEach(() => {
    document.body.innerHTML = `
      <main class="app">
        <div id="status"></div>
        <svg id="board" viewBox="0 0 900 780"></svg>
        <button id="undo-move"></button>
        <button id="swap-move"></button>
        <button id="new-game"></button>
        <button id="ai-move"></button>
        <button id="ai-first-move"></button>
        <select id="ai-strength"><option value="2" selected>Strong</option></select>
      </main>
    `;
  });

  it('makes an AI move after the player clicks a cell', async () => {
    await import('../src/main.js');

    const humanCell = document.querySelector('polygon[data-idx="0"]');
    const aiCell = document.querySelector('polygon[data-idx="1"]');

    humanCell.dispatchEvent(new MouseEvent('click', { bubbles: true }));
    await new Promise((resolve) => setTimeout(resolve, 30));

    expect(aiCell.classList.contains('p1')).toBe(true);
  });
});
