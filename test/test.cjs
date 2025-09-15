const assert = require('node:assert');
const fs = require('node:fs');
const path = require('node:path');
const { loads } = require('../dist/index.cjs');

const GOLDEN = JSON.parse(fs.readFileSync(path.join(__dirname, '..', 'tests', 'golden.json'), 'utf8'));

function resolve(obj, pointer) {
  const parts = pointer.split('/').slice(1).map(p => p.replace(/~1/g, '/').replace(/~0/g, '~'));
  let cur = obj;
  for (const part of parts) {
    if (Array.isArray(cur)) {
      cur = cur[Number(part)];
    } else {
      cur = cur[part];
    }
  }
  return cur;
}

for (const [name, caseData] of Object.entries(GOLDEN.correct)) {
  const obj = loads(JSON.stringify(caseData.doc));
  for (const group of caseData.aliases) {
    const targets = group.map(p => resolve(obj, p));
    const first = targets[0];
    for (const t of targets.slice(1)) {
      assert.strictEqual(t, first);
    }
  }
  if (caseData['expect-keys']) {
    for (const [p, keys] of Object.entries(caseData['expect-keys'])) {
      const target = resolve(obj, p);
      for (const key of keys) {
        assert.ok(Object.prototype.hasOwnProperty.call(target, key));
      }
    }
  }
}

for (const [name, caseData] of Object.entries(GOLDEN.invalid)) {
  let doc = { ...caseData.doc };
  if (name === 'ref-with-extras') {
    const id = Object.values(doc)[0]['#'];
    doc = { owner: { '#': id, v: 0 }, ...doc };
  }
  assert.throws(() => loads(JSON.stringify(doc)));
}

console.log('tests passed');
