const assert = require('node:assert');
const { dumps, loads } = require('../dist/index.cjs');

function testSelfLoop() {
  const a = {};
  a.self = a;
  const data = dumps(a);
  const b = loads(data);
  assert.strictEqual(b, b.self);
}

function testSharedSubobject() {
  const left = {};
  const right = {};
  left.buddy = right;
  right.buddy = left;
  const root = { left, right };
  const data = dumps(root);
  const obj = loads(data);
  assert.strictEqual(obj.left.buddy, obj.right);
  assert.strictEqual(obj.right.buddy, obj.left);
}

testSelfLoop();
testSharedSubobject();

console.log('tests passed');
