import { execSync } from 'node:child_process';
import { existsSync, mkdirSync } from 'node:fs';

const run = (cmd, opts = {}) => {
  console.log(`> ${cmd}`);
  execSync(cmd, { stdio: 'inherit', ...opts });
};

const hasCommand = (cmd) => {
  try {
    execSync(`command -v ${cmd}`, { stdio: 'ignore', shell: '/bin/bash' });
    return true;
  } catch {
    return false;
  }
};

try {
  run('rustup target add wasm32-unknown-unknown');
} catch {
  console.warn('Could not add wasm target automatically; assuming it already exists.');
}

if (!hasCommand('wasm-bindgen')) {
  console.log('wasm-bindgen not found; installing wasm-bindgen-cli via cargo install...');
  run('cargo install wasm-bindgen-cli');
}

run('cargo build --manifest-path crate/Cargo.toml --lib --target wasm32-unknown-unknown --release');

if (!existsSync('src/wasm')) {
  mkdirSync('src/wasm', { recursive: true });
}

run('wasm-bindgen crate/target/wasm32-unknown-unknown/release/yavalath_engine.wasm --out-dir src/wasm --target web');
