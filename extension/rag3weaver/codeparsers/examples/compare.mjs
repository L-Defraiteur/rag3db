import { readFileSync } from 'fs';

const rust = JSON.parse(readFileSync('/tmp/rust_output.json', 'utf8'));
const ts = JSON.parse(readFileSync('/tmp/ts_output.json', 'utf8'));

// --- SCOPES ---
const allFiles = new Set([...Object.keys(rust.files), ...Object.keys(ts.files)]);

console.log('=== FILES ===');
console.log(`Rust: ${Object.keys(rust.files).length} files, TS: ${Object.keys(ts.files).length} files`);

const rustOnly = [...allFiles].filter(f => !ts.files[f]);
const tsOnly = [...allFiles].filter(f => !rust.files[f]);
if (rustOnly.length) console.log('Only in Rust:', rustOnly);
if (tsOnly.length) console.log('Only in TS:', tsOnly);

console.log('\n=== SCOPE DIFFERENCES PER FILE ===');
let totalRS = 0, totalTS = 0;
for (const f of [...allFiles].sort()) {
  const rs = rust.files[f]?.scopes || [];
  const tsS = ts.files[f]?.scopes || [];
  totalRS += rs.length;
  totalTS += tsS.length;
  if (rs.length !== tsS.length) {
    console.log(`\n${f}: Rust=${rs.length} TS=${tsS.length} (${rs.length - tsS.length > 0 ? '+' : ''}${rs.length - tsS.length})`);
    const rNames = new Set(rs.map(s => s.name));
    const tNames = new Set(tsS.map(s => s.name));
    const onlyR = rs.filter(s => !tNames.has(s.name));
    const onlyT = tsS.filter(s => !rNames.has(s.name));
    if (onlyR.length) console.log('  Only in Rust:', onlyR.map(s => `${s.type}:${s.name}(L${s.startLine}-L${s.endLine})`));
    if (onlyT.length) console.log('  Only in TS:  ', onlyT.map(s => `${s.type}:${s.name}(L${s.startLine}-L${s.endLine})`));
  }
}
console.log(`\nTotal scopes: Rust=${totalRS} TS=${totalTS} diff=${totalRS - totalTS}`);

// --- RELATIONSHIPS ---
console.log('\n=== RELATIONSHIPS BY TYPE ===');
const rRels = rust.relationships || [];
const tRels = ts.relationships || [];
console.log(`Rust: ${rRels.length}, TS: ${tRels.length}`);

const rByType = {}, tByType = {};
for (const r of rRels) rByType[r.type] = (rByType[r.type] || 0) + 1;
for (const r of tRels) tByType[r.type] = (tByType[r.type] || 0) + 1;
const allTypes = new Set([...Object.keys(rByType), ...Object.keys(tByType)]);
for (const t of [...allTypes].sort()) {
  const rc = rByType[t] || 0, tc = tByType[t] || 0;
  const diff = rc - tc;
  const marker = diff === 0 ? '=' : (diff > 0 ? `+${diff}` : `${diff}`);
  console.log(`  ${t}: Rust=${rc} TS=${tc} (${marker})`);
}

// --- SPECIFIC DIFFERENCES ---
const rKey = r => `${r.from}|${r.to}|${r.type}`;
const rSet = new Set(rRels.map(rKey));
const tSet = new Set(tRels.map(rKey));
const onlyInRust = [...rSet].filter(k => !tSet.has(k));
const onlyInTS = [...tSet].filter(k => !rSet.has(k));

console.log(`\n=== RELATIONSHIPS ONLY IN RUST (${onlyInRust.length}) ===`);
for (const k of onlyInRust.sort().slice(0, 50)) {
  const [from, to, type] = k.split('|');
  console.log(`  ${type}: ${from} -> ${to}`);
}
if (onlyInRust.length > 50) console.log(`  ... and ${onlyInRust.length - 50} more`);

console.log(`\n=== RELATIONSHIPS ONLY IN TS (${onlyInTS.length}) ===`);
for (const k of onlyInTS.sort().slice(0, 50)) {
  const [from, to, type] = k.split('|');
  console.log(`  ${type}: ${from} -> ${to}`);
}
if (onlyInTS.length > 50) console.log(`  ... and ${onlyInTS.length - 50} more`);
