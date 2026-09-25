// One-shot, read-only collection capture. Does not read account credentials.
const fs = require('node:fs');
const path = require('node:path');
const reader = require('mtga-reader');
const output = path.resolve(__dirname, '../data/collection.json');
const timer = setTimeout(() => {
  console.error('Lecture interrompue après 90 secondes. Vérifier Arena et les permissions.');
  process.exit(1);
}, 90000);
(async () => {
  const processName = 'MTGA';
  if (!await reader.findProcess(processName)) throw new Error('Arena invisible : lancer le jeu sur son accueil et exécuter hors du bac à sable.');
  const result = await reader.readCollection(processName);
  if (result.error) throw new Error(result.error);
  if (!Array.isArray(result.cards) || !result.cards.length) throw new Error('Collection vide : ancien export conservé.');
  const seen = new Set();
  for (const c of result.cards) {
    if (!Number.isSafeInteger(c.grpId) || c.grpId <= 0 || !Number.isSafeInteger(c.qty) || c.qty < 0 || seen.has(c.grpId)) throw new Error('Collection invalide');
    seen.add(c.grpId);
  }
  const payload = {captured_at: new Date().toISOString(), source: 'mtga-reader', ...result};
  fs.mkdirSync(path.dirname(output), {recursive: true});
  fs.writeFileSync(output + '.tmp', JSON.stringify(payload, null, 2));
  fs.renameSync(output + '.tmp', output);
  console.log(`${result.cards.length} identifiants, ${result.cards.reduce((s,c)=>s+c.qty,0)} exemplaires → ${output}`);
})().catch(e => {console.error(e.message); process.exitCode = 1;})
  .finally(() => {clearTimeout(timer); reader.close();});
