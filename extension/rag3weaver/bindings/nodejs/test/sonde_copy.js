'use strict';
// La sonde du COPY : une entité déclarée avec un DOUBLE et une liste de
// doubles, des lignes `score = num + 0.5`, ingérées par lots (le chemin de
// masse du moteur, COPY journalisé, plusieurs fils), puis comptées par le
// moteur lui-même : zéro ligne fausse. Le 11 octobre 2026, l'arbre principal
// a reproduit en 9 s une colonne DOUBLE corrompue en mémoire sous deux fils
// avec une colonne tableau — le chemin de rag3weaver. Cette sonde tourne sur
// les trois plateformes avec le binaire livré.
//
//   SONDE_LIGNES   nombre de lignes (défaut 200 000 ; 20 000 sous SONDE_REDUITE=1)
//   SONDE_LOT      lignes par appel d'ingestion (défaut 512, le plafond de l'outil)
//   SONDE_VERBEUX=1  imprime ce que le backend a dit du COPY sur stderr
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const { Backend, binaryPath, vectorExtensionPath, templatesDir, prepareManifest } = require('..');

const lignes = Number(process.env.SONDE_LIGNES || (process.env.SONDE_REDUITE === '1' ? 20000 : 200000));
const lot = Number(process.env.SONDE_LOT || 512); // records : 512 au plus par appel (schéma d'entrée de l'outil)

async function main() {
  const tmp = fs.mkdtempSync(path.join(os.tmpdir(), 'rag3weaver-sonde-'));
  const ws = path.join(tmp, 'ws');
  fs.mkdirSync(ws);
  fs.writeFileSync(path.join(ws, 'main.rs'), 'fn main() {}\n');
  fs.writeFileSync(path.join(tmp, 'mesure.json'), JSON.stringify({
    type: 'object', additionalProperties: false,
    required: ['cle', 'num', 'text', 'score', 'vecteur'],
    properties: {
      cle: { type: 'string', minLength: 1 }, num: { type: 'integer' }, text: { type: 'string', minLength: 1 },
      score: { type: 'number' }, vecteur: { type: 'array', items: { type: 'number' } },
    },
  }));
  // La relecture : les lignes de Mesure par pages de `num` (SelectRecordsNode
  // sans limite sur la page), comparées ici — un CypherNode brut n'est pas
  // un nœud d'outil de backend, et le RhaiNode ne prend pas des résultats.
  fs.writeFileSync(path.join(tmp, 'sonde_lire.mmd'), [
    '%% tool: sonde_lire',
    '%% description: Relit les lignes de Mesure qui passent le filtre.',
    '%% param: filter json! -- Le filtre de sélection.',
    '%% result: sel.results',
    '',
    'graph LR',
    '    sel["SelectRecordsNode(entity=\'Mesure\', filter=$filter, limit=0)"]',
    '',
  ].join('\n'));
  const page = Number(process.env.SONDE_PAGE || 5000);

  const manifest = prepareManifest(path.join(templatesDir(), 'backends', 'code', 'backend.json'), {
    name: 'sonde-copy',
    database: path.join(tmp, 'sonde.rag3db'),
    vector_extension: vectorExtensionPath(),
    workspace: { root: ws, commands: 'off', sandbox: { mode: 'off' } },
    tools: {
      run_command: undefined,
      sonde_ingest: { graph: path.join(templatesDir(), 'tools', 'ingest_snapshot.mmd'), bindings: { entity: 'Mesure' } },
      sonde_lire: { graph: path.join(tmp, 'sonde_lire.mmd') },
    },
    entities: {
      Mesure: {
        schema: path.join(tmp, 'mesure.json'),
        config: {
          fields: {
            cle: { type: 'string', isTitle: true },
            num: { type: 'int64' },
            text: { type: 'string', isContent: true },
            score: { type: 'double' },
            vecteur: { type: { list: 'double' } },
          },
          hashsafe: ['cle'],
          signals: ['bm25'],
          chunked: false,
        },
      },
    },
  });
  const manifestPath = path.join(tmp, 'backend.json');
  fs.writeFileSync(manifestPath, JSON.stringify(manifest, null, 2));

  console.log(`binaire : ${binaryPath()}`);
  const backend = await Backend.open(manifestPath);
  const t0 = Date.now();
  let ecrites = 0;
  for (let debut = 0; debut < lignes; debut += lot) {
    const records = [];
    for (let i = debut; i < Math.min(debut + lot, lignes); i++) {
      records.push({ cle: `m${i}`, num: i, text: `ligne ${i}`, score: i + 0.5, vecteur: [i, i / 2, 1.5, -i] });
    }
    const r = await backend.call('sonde_ingest', { records, snapshot: '' });
    if (r && r.ok === false) throw new Error(`ingestion refusée au lot ${debut} : ${JSON.stringify(r).slice(0, 400)}`);
    ecrites += records.length;
  }
  console.log(`${ecrites} lignes ingérées en ${((Date.now() - t0) / 1000).toFixed(1)} s (lots de ${lot})`);

  const t1 = Date.now();
  let total = 0, scoreFaux = 0, vecteurFaux = 0;
  const vus = new Set();
  for (let debut = 0; debut < lignes; debut += page) {
    const r = await backend.call('sonde_lire', {
      filter: { field: { key: 'num', value: [{ op: 'gte', value: debut }, { op: 'lt', value: debut + page }] } },
    });
    const rows = Array.isArray(r.result) ? r.result : (Array.isArray(r) ? r : null);
    if (!rows) throw new Error(`sonde_lire ne rend pas de lignes : ${JSON.stringify(r).slice(0, 400)}`);
    for (const row of rows) {
      const d = row.data || row;
      const num = Number(d.num);
      if (vus.has(num)) throw new Error(`ligne ${num} rendue deux fois`);
      vus.add(num);
      total += 1;
      if (Math.abs(Number(d.score) - (num + 0.5)) > 1e-6) { scoreFaux += 1; if (scoreFaux <= 3) console.log(`  score faux : num ${num} → ${d.score}`); }
      const v = d.vecteur;
      if (!Array.isArray(v) || v.length !== 4 || Math.abs(Number(v[0]) - num) > 1e-6 || Math.abs(Number(v[3]) + num) > 1e-6) {
        vecteurFaux += 1; if (vecteurFaux <= 3) console.log(`  vecteur faux : num ${num} → ${JSON.stringify(v)}`);
      }
    }
  }
  console.log(`relecture par pages de ${page} en ${((Date.now() - t1) / 1000).toFixed(1)} s`);
  if (total !== lignes) throw new Error(`total ${total} ≠ ${lignes} lignes écrites`);
  if (scoreFaux !== 0 || vecteurFaux !== 0) {
    throw new Error(`COPY faux : ${scoreFaux} score(s) ≠ num + 0.5, ${vecteurFaux} vecteur(s) faux sur ${total}`);
  }
  console.log(`sonde : ${total} lignes relues, score = num + 0.5 et vecteur justes partout`);

  if (process.env.SONDE_VERBEUX === '1') {
    console.log('--- stderr du backend (fin) ---');
    console.log(backend.stderr.join('').split('\n').filter((l) => /copy|COPY|lot|batch/.test(l)).slice(-12).join('\n'));
  }
  const exit = await backend.shutdown();
  console.log(`arrêt propre : code ${exit.code}`);
  fs.rmSync(tmp, { recursive: true, force: true });
  console.log('OK');
}

main().catch((e) => { console.error(`ÉCHEC : ${e.message}`); process.exit(1); });
