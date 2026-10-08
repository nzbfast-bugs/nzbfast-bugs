// Placeholder-substitution tests for t() (`node web/subst_test.js`).
//
// Lifts t() straight out of web/dashboard.html and web/wall.html. Pins
// #322: substitution is ONE pass over the template, so a {token} inside a
// value (a release name is user-controlled text) is never re-scanned and
// replaced by a later parameter.
const fs = require('fs');
const path = require('path');

function lift(page, name) {
  const at = page.indexOf('function ' + name + '(');
  if (at < 0) throw new Error('no function ' + name);
  let i = page.indexOf('{', at), depth = 0;
  for (let j = i; j < page.length; j++) {
    if (page[j] === '{') depth++;
    else if (page[j] === '}' && --depth === 0) return page.slice(at, j + 1);
  }
  throw new Error('unbalanced ' + name);
}

let fails = 0;
const eq = (got, want, msg) => {
  if (got !== want) { fails++; console.log(`FAIL ${msg}\n  got:  ${got}\n  want: ${want}`); }
};

for (const file of ['dashboard.html', 'wall.html']) {
  const page = fs.readFileSync(path.join(__dirname, file), 'utf8');
  const t = new Function('I18N', lift(page, 't') + '\nreturn t;')({});
  eq(t('toast.completedMoved', 'Completed: {name} - moved to {dest}',
        { name: 'Show.{dest}.S01E01', dest: '/mnt/tv' }),
     'Completed: Show.{dest}.S01E01 - moved to /mnt/tv', `${file} value re-scanned`);
  eq(t('x', '{a} and {b}', { a: '{b}', b: '{a}' }), '{b} and {a}', `${file} swap`);
  eq(t('x', '{n} of {n}', { n: 3 }), '3 of 3', `${file} repeated token`);
  eq(t('x', 'keep {other}', { n: 1 }), 'keep {other}', `${file} unknown token kept`);
  eq(t('x', 'no params {n}'), 'no params {n}', `${file} no params`);
  eq(t('x', '{toString}', {}), '{toString}', `${file} prototype key not substituted`);
}
if (fails) { console.log(fails + ' failure(s)'); process.exit(1); }
console.log('subst_test: ok');
