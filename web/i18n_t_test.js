// Unit test for the dashboard's t() placeholder substitution
// (`node web/i18n_t_test.js`). Lifts t() straight out of dashboard.html.
//
// A substituted value must never be rescanned for placeholders: a release
// named "Some.Release.{why}" shown through 'hist.alt.from' must render its
// own name verbatim, not with the {why} param pasted into it.
const fs = require('fs');
const assert = require('assert');
const page = fs.readFileSync(__dirname + '/dashboard.html', 'utf8');
function lift(name) {
  const at = page.indexOf('function ' + name + '(');
  if (at < 0) throw new Error('no function ' + name + ' in dashboard.html');
  let i = page.indexOf('{', at), depth = 0, end = -1;
  for (let j = i; j < page.length; j++) {
    if (page[j] === '{') depth++;
    else if (page[j] === '}' && --depth === 0) { end = j + 1; break; }
  }
  return page.slice(at, end);
}
const t = new Function('I18N', lift('t') + '; return t;')({});

assert.strictEqual(
  t('hist.alt.from', '{name}, abandoned because: {why}', {name: 'Some.Release.{why}', why: 'stalled'}),
  'Some.Release.{why}, abandoned because: stalled');
assert.strictEqual(t('k', '{a} and {b}', {a: '{b}', b: 'x'}), '{b} and x');
// Ordinary behaviour unchanged.
assert.strictEqual(t('k', '{n} of {m}', {n: 1, m: 2}), '1 of 2');
assert.strictEqual(t('k', '{n} {n}', {n: 3}), '3 3');
assert.strictEqual(t('k', 'keep {unknown}', {n: 3}), 'keep {unknown}');
assert.strictEqual(t('k', 'plain'), 'plain');
console.log('i18n_t_test: ok');
