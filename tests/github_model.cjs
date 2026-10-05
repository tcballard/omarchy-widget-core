const assert=require('node:assert/strict');
const m=require('../widgets/github-contributions/Model.js');
assert(m.validUsername('tcballard'));assert(!m.validUsername('https://github.com'));
let days=Array.from({length:366},(_,i)=>({date:new Date(Date.UTC(2025,9,5+i)).toISOString().slice(0,10),weekday:i%7,count:i%5,level:i%5}));
assert.equal(m.weeks(days).length,53);
for (let family of ['medium','large']) {
  let flattened=m.sections(days,family).flat(2).filter(Boolean);
  assert.deepEqual(flattened,days);
}
assert.equal(m.sections(days,'small')[0].length,13);
assert.equal(m.weeks([{date:'2026-01-07',weekday:3,count:0,level:0}])[0][0],null);
assert.equal(m.describe({date:'2026-10-05',count:1}),'1 contribution · 2026-10-05');
assert.deepEqual(m.sections([], 'medium'),[[]]);
console.log('PASS: exact daily alignment, family windows, partial weeks and input validation');
