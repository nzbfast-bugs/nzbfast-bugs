// Plural-selection tests for tn() (`node web/plural_test.js`).
//
// Lifts t()/tn() straight out of web/dashboard.html and web/wall.html (no
// copy to drift) and runs them against the real catalogues. Pins #321:
// CLDR's "one" category is not "exactly 1" - ru/uk/hr/sr put 21 and 101
// there, sl puts 101 (and 102 in its dual), fr/pt put 0 - and the catalogues' count-less .one
// strings ("remove the selected entry …") must never be shown for those
// counts, or a 21-item destructive confirm reads as a single entry.
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
const check = (ok, msg) => { if (!ok) { fails++; console.log('FAIL ' + msg); } };

for (const file of ['dashboard.html', 'wall.html']) {
  const page = fs.readFileSync(path.join(__dirname, file), 'utf8');
  const src = lift(page, 't') + '\n' + lift(page, 'tn') + '\nreturn tn;';
  for (const L of ['en', 'ru', 'uk', 'hr', 'sr', 'sl', 'fr', 'pt', 'pl', 'de']) {
    const I18N = L === 'en' ? {} :
      JSON.parse(fs.readFileSync(path.join(__dirname, 'i18n', L + '.json'), 'utf8'));
    const tn = new Function('I18N', '_pr', src)(I18N, new Intl.PluralRules(L));
    for (const n of [0, 1, 2, 5, 21, 22, 101, 102]) {
      for (const k of ['confirm.sel.hfiles', 'add.title.n']) {
        const s = tn(k, n, 'ONE', '{n} MANY');
        // A count-less singular (n=1) or dual (n=2, sl/he/ar) is exact.
        const cat = new Intl.PluralRules(L).select(n);
        if ((cat === 'one' && n === 1) || (cat === 'two' && n === 2)) continue;
        check(s.includes(String(n)), `${file} ${L} n=${n} ${k}: ${s.split('\n')[0]}`);
      }
    }
    // n === 1 still gets the singular sentence.
    const one = tn('confirm.sel.hfiles', 1, 'ONE', '{n} MANY');
    check(L === 'en' ? one === 'ONE' : one === I18N['confirm.sel.hfiles.one'],
      `${file} ${L} n=1 lost its singular: ${one}`);
  }
}
if (fails) { console.log(fails + ' failure(s)'); process.exit(1); }
console.log('plural_test: ok');
