#!/usr/bin/env node
import { cpSync, existsSync, mkdirSync, rmSync } from 'node:fs';
import { dirname, resolve, sep } from 'node:path';
import { fileURLToPath } from 'node:url';
import { spawnSync } from 'node:child_process';

const scriptDir = dirname(fileURLToPath(import.meta.url));
const repoRoot = resolve(scriptDir, '..');

function toWebPath(value) {
  return value.split(sep).join('/');
}

function normalizeBasePath(basePath) {
  let normalized = basePath.trim();
  if (!normalized.startsWith('/')) normalized = `/${normalized}`;
  if (!normalized.endsWith('/')) normalized = `${normalized}/`;
  return normalized;
}

function run(cmd, args) {
  const result = spawnSync(cmd, args, {
    cwd: repoRoot,
    stdio: 'inherit',
    env: process.env,
  });

  if (result.status !== 0) {
    process.exit(result.status ?? 1);
  }
}

const targetArg = process.argv[2];
const baseArg = process.argv[3];

if (!targetArg) {
  console.error('Usage: node scripts/build-for-subdir.mjs <target-path> [base-path]');
  console.error('Example: node scripts/build-for-subdir.mjs ./random /random/');
  process.exit(1);
}

const targetPath = resolve(repoRoot, targetArg);
const cleanedTarget = toWebPath(targetArg).replace(/^(\.\/)+/, '').replace(/^\/+/, '').replace(/\/+$/, '');
const defaultBase = cleanedTarget ? `/${cleanedTarget}/` : '/';
const basePath = normalizeBasePath(baseArg || defaultBase);
const distPath = resolve(repoRoot, 'dist');

if (existsSync(distPath)) {
  rmSync(distPath, { recursive: true, force: true });
}

console.log(`Building with base path: ${basePath}`);
run('npm', ['run', 'build', '--', '--base', basePath]);

if (!existsSync(distPath)) {
  console.error('Build did not produce dist/.');
  process.exit(1);
}

rmSync(targetPath, { recursive: true, force: true });
mkdirSync(targetPath, { recursive: true });
cpSync(distPath, targetPath, { recursive: true });

console.log(`\n✅ Site files copied to: ${targetPath}`);
console.log(`Serve ${basePath} from GitHub Pages to load this build.`);
