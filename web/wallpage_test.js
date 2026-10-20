// Unit test for the wall's infinite-scroll paging (`node web/wallpage_test.js`).
//
// Same approach as web/fmt_test.js: wallFetch is lifted straight out of
// web/wall.html and run against a fake `wall2` that serves offset/limit
// slices of a list, so the copy here cannot drift from the one that ships.
//
// What it pins (GH #325): releases that arrive while the reader is
// scrolled down only raise the "new arrivals" pill - the wall is not
// reset - but they push the server's list down. The next page used to be
// fetched at offset=cards.length and re-sent cards already on the wall,
// so the same title showed twice. Now: no duplicates, no skips.
const fs = require('fs');
const page = fs.readFileSync(__dirname + '/wall.html', 'utf8');

function lift(name) {
  const at = page.indexOf('async function ' + name + '(');
  if (at < 0) throw new Error('no function ' + name + ' in wall.html');
  let i = page.indexOf('{', at), depth = 0;
  for (let j = i; j < page.length; j++) {
    if (page[j] === '{') depth++;
    else if (page[j] === '}' && --depth === 0) return page.slice(at, j + 1);
  }
  throw new Error('unbalanced body for ' + name);
}

let fails = 0;
function check(name, cond, detail) {
  if (cond) console.log('ok   ' + name);
  else { fails++; console.log('FAIL ' + name + (detail ? ': ' + detail : '')); }
}

function harness(db) {
  const S = { wseq: 0, wloading: false, cards: [], wtotal: 0, wsig: '', wskew: 0,
    idxGroups: 0, idxPaused: false, unmTotal: 0, unmBusy: false, dom: [] };
  const el = { value: '', textContent: '', innerHTML: '', style: {}, querySelector: () => null };
  const env = {
    document: { getElementById: () => el, querySelectorAll: () => [] },
    api: async (m, qs) => {
      const o = +/offset=(\d+)/.exec(qs)[1], l = +/limit=(\d+)/.exec(qs)[1];
      return { cards: db.slice(o, o + l), total: db.length, cats: {} };
    },
    tab: 'all', sortBy: 'arrived', curDesc: () => true, matchedOnly: false, catGroup: false,
    grpQS: () => '', genreSel: '', resSel: '', decSel: '', toast() {}, tErr: x => x,
    updateCats() {}, narrowsResults: () => false,
    render() { S.dom = S.cards.map(c => c.key); }, renderStrips() {},
    appendCards: p => S.dom.push(...p.map(c => c.key)),
    openKey: null, openCard() {}, t: (k, d) => d, tn: () => '',
    showSkeleton() {}, clearSkeleton() {}, maybeMore() {},
  };
  const names = Object.keys(env);
  // Module-level state becomes fields of S; `j.cards` and object keys stay.
  const src = lift('wallFetch').replace(
    /(?<![.\w])(wseq|wloading|cards|wtotal|wsig|wskew|idxGroups|idxPaused|unmTotal|unmBusy)\b(?!:)/g, 'S.$1');
  const fn = new Function('S', ...names, 'const WPAGE=60;return (' + src + ')')(S, ...names.map(n => env[n]));
  return { S, wallFetch: fn };
}

const dups = keys => keys.filter((k, i) => keys.indexOf(k) !== i);

(async () => {
  // 1. Five arrivals between page 1 and page 2, three more before page 3.
  {
    const db = [];
    for (let i = 0; i < 200; i++) db.push({ key: 'k' + (200 - i) });
    const original = db.map(c => c.key);
    const { S, wallFetch } = harness(db);
    await wallFetch(true);
    for (let i = 0; i < 5; i++) db.unshift({ key: 'a' + i });
    await wallFetch(false);
    for (let i = 0; i < 3; i++) db.unshift({ key: 'b' + i });
    await wallFetch(false);
    check('no duplicate cards across pages', dups(S.dom).length === 0, dups(S.dom).join(','));
    // 60 + 55 (5 re-sent) + 57 (3 re-sent): an unbroken run of the
    // original order, nothing missing in between.
    const want = original.slice(0, 172).join(',');
    check('no card skipped: pages continue in order', S.dom.join(',') === want,
      'got ' + S.dom.length + ' cards');
  }
  // 2. A whole page's worth of arrivals: the next page is all duplicates,
  //    and the scroll after it must still move on rather than stall.
  {
    const db = [];
    for (let i = 0; i < 200; i++) db.push({ key: 'k' + (200 - i) });
    const original = db.map(c => c.key);
    const { S, wallFetch } = harness(db);
    await wallFetch(true);
    for (let i = 0; i < 60; i++) db.unshift({ key: 'a' + i });
    await wallFetch(false);
    await wallFetch(false);
    check('all-duplicate page does not stall paging',
      S.dom.join(',') === original.slice(0, 120).join(','), 'got ' + S.dom.length + ' cards');
  }
  // 3. A reset re-reads from the top and forgets the skew.
  {
    const db = [];
    for (let i = 0; i < 200; i++) db.push({ key: 'k' + (200 - i) });
    const { S, wallFetch } = harness(db);
    await wallFetch(true);
    for (let i = 0; i < 5; i++) db.unshift({ key: 'a' + i });
    await wallFetch(false);
    await wallFetch(true);
    await wallFetch(false);
    check('reset clears the skew', S.dom.join(',') === db.slice(0, 120).map(c => c.key).join(','));
  }
  if (fails) { console.log(fails + ' failed'); process.exit(1); }
  console.log('all passed');
})();
