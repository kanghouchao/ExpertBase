// KB パイプラインの決定的ヘルパ。スキルが呼ぶ機械処理だけを担い、判断は LLM 側。
// 使い方: node scripts/kb-pipeline.mjs <stale|todo|skeleton|slug|set-last-run> <kb-root> [args]
import fs from 'node:fs';
import path from 'node:path';
import crypto from 'node:crypto';
import { pathToFileURL } from 'node:url';

// --- 純関数（テスト対象） -------------------------------------------------

/** frontmatter 本文の内容ハッシュ。staleness 判定の唯一の根拠。 */
export function sourceHash(body) {
  return crypto.createHash('sha256').update(body, 'utf8').digest('hex').slice(0, 16);
}

/** `---\n...\n---\n` を yaml と本文に分割する。frontmatter が無ければ body 全体。 */
export function splitFrontmatter(raw) {
  const text = raw.startsWith('﻿') ? raw.slice(1) : raw;
  if (!text.startsWith('---\n')) return { yaml: '', body: text };
  const end = text.indexOf('\n---', 4);
  if (end === -1) return { yaml: '', body: text };
  const yaml = text.slice(4, end);
  let body = text.slice(end + 4);
  body = body.replace(/^\r?\n/, '').replace(/^\r?\n/, '');
  return { yaml, body };
}

/** entries ファイル名 slug。Rust kb::store::slug と一致させる。 */
export function slug(title) {
  const cleaned = [...title].map((c) => ('/\\:*?"<>|'.includes(c) ? '-' : c)).join('');
  const trimmed = cleaned.trim().replaceAll(' ', '-');
  return trimmed === '' ? 'untitled' : trimmed;
}

// --- 内部ユーティリティ ---------------------------------------------------

const yamlField = (yaml, key) => {
  const m = yaml.match(new RegExp(`^${key}:\\s*(.+)$`, 'm'));
  return m ? m[1].trim() : null;
};
const read = (root, rel) => fs.readFileSync(path.join(root, rel), 'utf8');
const listMd = (root, rel, suffix) => {
  const dir = path.join(root, rel);
  if (!fs.existsSync(dir)) return [];
  return fs.readdirSync(dir)
    .filter((f) => f.endsWith(suffix))
    .map((f) => `${rel}/${f}`);
};
const summaryRelFor = (inboxRel) =>
  `wiki/summaries/${path.basename(inboxRel, '.md')}.summary.md`;

const statePath = (root) => path.join(root, '.expertbase', 'state.json');
const readState = (root) =>
  fs.existsSync(statePath(root)) ? JSON.parse(fs.readFileSync(statePath(root), 'utf8')) : {};
const saveState = (root, state) => {
  fs.mkdirSync(path.dirname(statePath(root)), { recursive: true });
  fs.writeFileSync(statePath(root), JSON.stringify(state, null, 2) + '\n');
};

// --- サブコマンド ---------------------------------------------------------

function cmdStale(root) {
  const out = [];
  for (const inboxRel of listMd(root, 'inbox', '.md')) {
    const body = splitFrontmatter(read(root, inboxRel)).body.trim();
    const summaryRel = summaryRelFor(inboxRel);
    const summaryFile = path.join(root, summaryRel);
    if (!fs.existsSync(summaryFile)) { out.push(inboxRel); continue; }
    const stored = yamlField(splitFrontmatter(read(root, summaryRel)).yaml, 'hash');
    if (stored !== sourceHash(body)) out.push(inboxRel);
  }
  return out;
}

function cmdTodo(root) {
  const lastRun = readState(root)?.concept?.last_run_at ?? null;
  const lastMs = lastRun ? Date.parse(lastRun) : null;
  const out = [];
  for (const rel of listMd(root, 'wiki/summaries', '.summary.md')) {
    if (lastMs === null) { out.push(rel); continue; }
    const at = yamlField(splitFrontmatter(read(root, rel)).yaml, 'summarized_at');
    if (!at || Date.parse(at) > lastMs) out.push(rel);
  }
  return out;
}

function cmdSkeleton(root, inboxRel) {
  const body = splitFrontmatter(read(root, inboxRel)).body.trim();
  const rel = summaryRelFor(inboxRel);
  fs.mkdirSync(path.dirname(path.join(root, rel)), { recursive: true });
  const fm =
    `---\n` +
    `type: summary\n` +
    `source: ${inboxRel}\n` +
    `hash: ${sourceHash(body)}\n` +
    `summarized_at: ${new Date().toISOString()}\n` +
    `tags: []\n` +
    `---\n\n` +
    `# \n\n## Summary\n\n## Key Concepts\n\n## Notable Details\n`;
  fs.writeFileSync(path.join(root, rel), fm);
  return rel;
}

function cmdSetLastRun(root, skill) {
  if (skill !== 'concept') throw new Error(`unknown skill: ${skill}`);
  const state = readState(root);
  state.concept = { ...(state.concept ?? {}), last_run_at: new Date().toISOString() };
  saveState(root, state);
  return state.concept.last_run_at;
}

// --- エントリポイント -----------------------------------------------------

function main(argv) {
  const [sub, ...rest] = argv;
  switch (sub) {
    case 'stale': return console.log(JSON.stringify(cmdStale(rest[0]), null, 2));
    case 'todo': return console.log(JSON.stringify(cmdTodo(rest[0]), null, 2));
    case 'skeleton': return console.log(cmdSkeleton(rest[0], rest[1]));
    case 'slug': return console.log(slug(rest[0] ?? ''));
    case 'set-last-run': return console.log(cmdSetLastRun(rest[0], rest[1]));
    default:
      console.error('Usage: node scripts/kb-pipeline.mjs <stale|todo|skeleton|slug|set-last-run> <kb-root> [args]');
      process.exit(1);
  }
}

if (import.meta.url === pathToFileURL(process.argv[1]).href) {
  main(process.argv.slice(2));
}
