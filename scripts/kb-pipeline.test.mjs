import { test } from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { execFileSync } from 'node:child_process';
import { sourceHash, splitFrontmatter, slug } from './kb-pipeline.mjs';

const CLI = path.join(import.meta.dirname, 'kb-pipeline.mjs');
const run = (root, ...args) =>
  execFileSync('node', [CLI, args[0], root, ...args.slice(1)], { encoding: 'utf8' });

function tmpKb() {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'ebkb-'));
  fs.mkdirSync(path.join(root, 'inbox'), { recursive: true });
  return root;
}
function writeInbox(root, name, body, frontmatter = 'type: text\nstatus: pending') {
  fs.writeFileSync(path.join(root, 'inbox', name), `---\n${frontmatter}\n---\n\n${body}\n`);
}

test('sourceHash is stable and content-sensitive', () => {
  assert.equal(sourceHash('hello'), sourceHash('hello'));
  assert.notEqual(sourceHash('hello'), sourceHash('hello!'));
});

test('splitFrontmatter separates yaml and body', () => {
  const { yaml, body } = splitFrontmatter('---\ntype: text\n---\n\nbody text\n');
  assert.match(yaml, /type: text/);
  assert.equal(body.trim(), 'body text');
});

test('slug mirrors Rust rule', () => {
  assert.equal(slug('a/b:c'), 'a-b-c');
  assert.equal(slug('  hello world  '), 'hello-world');
  assert.equal(slug('緑茶'), '緑茶');
  assert.equal(slug('   '), 'untitled');
});

test('stale lists materials with no summary', () => {
  const root = tmpKb();
  writeInbox(root, 'a.md', 'alpha');
  assert.deepEqual(JSON.parse(run(root, 'stale')), ['inbox/a.md']);
});

test('stale skips up-to-date summaries and catches edits', () => {
  const root = tmpKb();
  writeInbox(root, 'a.md', 'alpha');
  run(root, 'skeleton', 'inbox/a.md');
  assert.deepEqual(JSON.parse(run(root, 'stale')), []); // hash matches
  writeInbox(root, 'a.md', 'alpha edited');             // body changed
  assert.deepEqual(JSON.parse(run(root, 'stale')), ['inbox/a.md']);
});

test('skeleton writes summary with source and hash', () => {
  const root = tmpKb();
  writeInbox(root, 'a.md', 'alpha');
  const out = run(root, 'skeleton', 'inbox/a.md').trim();
  assert.equal(out, 'wiki/summaries/a.summary.md');
  const text = fs.readFileSync(path.join(root, out), 'utf8');
  assert.match(text, /source: inbox\/a\.md/);
  assert.match(text, new RegExp(`hash: ${sourceHash('alpha')}`));
  assert.match(text, /## Summary/);
  assert.match(text, /## Key Concepts/);
});

test('todo respects concept last_run_at', () => {
  const root = tmpKb();
  fs.mkdirSync(path.join(root, 'wiki', 'summaries'), { recursive: true });
  const mk = (name, at) =>
    fs.writeFileSync(
      path.join(root, 'wiki', 'summaries', name),
      `---\ntype: summary\nsource: inbox/${name}\nsummarized_at: ${at}\n---\n\n## Summary\n`,
    );
  mk('a.summary.md', '2026-06-21T00:00:00Z');
  assert.deepEqual(JSON.parse(run(root, 'todo')), ['wiki/summaries/a.summary.md']); // first run: all
  run(root, 'set-last-run', 'concept');
  assert.deepEqual(JSON.parse(run(root, 'todo')), []); // nothing newer
  mk('b.summary.md', '2999-01-01T00:00:00Z');
  assert.deepEqual(JSON.parse(run(root, 'todo')), ['wiki/summaries/b.summary.md']);
});
