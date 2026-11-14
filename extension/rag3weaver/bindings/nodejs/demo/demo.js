'use strict';
// La démo du paquet : depuis un dossier vide où `rag3weaver` est installé,
// ouvrir un backend de code sur un dépôt, l'indexer, et montrer ce qu'on sait
// faire — en texte lisible, pas en JSON brut. Lancée par demo.sh, deux fois :
// avec un service d'embarquement (recherche hybride, mots + sens) et sans
// (plein texte seul, l'avertissement visible). Chaque passe dit quel
// embarqueur a calculé ses vecteurs : le modèle et l'adresse, lus au /sante
// du service, ou « aucun ».
//
//   node demo.js <dossier du dépôt> [sans-service]
//
// Les variables : RAG3WEAVER_SERVICE_EMBED (alias RAG3WEAVER_EMBED_SERVICE)
// pour le service ; DEMO_QUERY, DEMO_SYMBOL pour changer la question et le
// symbole.
const fs = require('node:fs');
const http = require('node:http');
const os = require('node:os');
const path = require('node:path');
const { Backend, binaryPath, vectorExtensionPath, templatesDir, prepareManifest } = require('rag3weaver');

const depot = path.resolve(process.argv[2] || '.');
const sansService = process.argv[3] === 'sans-service';
const question = process.env.DEMO_QUERY || 'comment les relations entre symboles sont-elles résolues ?';
const symbole = process.env.DEMO_SYMBOL || 'resolve_relationships';

const gras = (t) => `\x1b[1m${t}\x1b[0m`;
const gris = (t) => `\x1b[2m${t}\x1b[0m`;
const titre = (t) => console.log(`\n${gras('━━ ' + t)}\n`);
const dormir = (ms) => new Promise((r) => setTimeout(r, ms));

// Les variables de service que le binaire lit : RAG3WEAVER_SERVICE_<CAPACITÉ>
// pour chaque capacité (le creux, le relecteur et l'OCR lisent aussi celle de
// l'embarquement), puis l'alias d'avant, RAG3WEAVER_EMBED_SERVICE. Sans
// service, la démo les retire TOUTES : le 14 novembre 2026, elle ne retirait
// que l'alias, et la variable commune posée par le poste embarquait quand même.
const estVariableDeService = (k) => k.startsWith('RAG3WEAVER_SERVICE_') || k === 'RAG3WEAVER_EMBED_SERVICE';
const VARIABLES_EMBED = ['RAG3WEAVER_SERVICE_EMBED', 'RAG3WEAVER_EMBED_SERVICE'];

// Les adresses du service, comme le binaire les lit : la première variable
// posée et non vide, dans l'ordre.
function adressesDuService() {
  for (const variable of VARIABLES_EMBED) {
    const adresses = String(process.env[variable] || '').split(',').map((a) => a.trim()).filter(Boolean);
    if (adresses.length) return { variable, adresses };
  }
  return null;
}

// L'identité d'un service d'embarquement, par son /sante ; null s'il ne répond pas.
function sante(adresse) {
  return new Promise((resolve) => {
    const req = http.get(`http://${adresse}/sante`, { timeout: 3000 }, (res) => {
      let corps = '';
      res.setEncoding('utf8');
      res.on('data', (c) => { corps += c; });
      res.on('end', () => { try { resolve(JSON.parse(corps)); } catch { resolve(null); } });
    });
    req.on('timeout', () => req.destroy());
    req.on('error', () => resolve(null));
  });
}

// Quel embarqueur calcule les vecteurs du modèle déclaré : l'adresse que la
// déclaration donne, sinon celles de la variable, et la première qui sert ce
// modèle d'après son /sante — la règle du binaire.
async function embarqueur(embeddings) {
  const modele = embeddings && embeddings.model;
  const declare = embeddings && embeddings.address;
  const s = declare && declare.length
    ? { variable: 'la déclaration', adresses: [].concat(declare) }
    : adressesDuService();
  if (!s) return 'aucun — aucune variable de service n’atteint le binaire : plein texte seul';
  for (const a of s.adresses) {
    const id = await sante(a);
    if (id && id.modele === modele) {
      return `${modele} servi par ${a} (${s.variable} ; /sante : ${id.service || '?'}, ${id.dim} dim.${id.adresse ? `, adresse ${id.adresse}` : ''})`;
    }
  }
  return `aucun — aucune adresse de ${s.variable} (${s.adresses.join(', ')}) ne sert ${modele} à son /sante`;
}

// Les vecteurs d'une entité, en un mot : « non déclarés » (Symbol, en BM25
// seul : ni une panne ni une dette), sinon le niveau et le pourcentage.
function vecteurs(v) {
  if (v.vectors === 'not_declared') return 'sans vecteurs (non déclarés)';
  return `vecteurs ${v.vectors}${v.vectors_percent != null ? ` (${v.vectors_percent}%)` : ''}`;
}

// Le rendu lisible d'une réponse d'outil : `presentation` quand le backend
// l'a composé, sinon `result`, sinon le JSON ; puis la section du crochet
// « after » (les Liens d'une recherche, l'impact d'un fichier lu), que le
// backend rend à part sous `after`.
function rendu(r, maxLignes = 60) {
  const texte = typeof r.presentation === 'string' ? r.presentation
    : typeof r.result === 'string' ? r.result
    : JSON.stringify(r, null, 2);
  const lignes = texte.split('\n');
  let sortie = lignes.length > maxLignes
    ? lignes.slice(0, maxLignes).join('\n') + gris(`\n… (${lignes.length - maxLignes} lignes de plus)`)
    : texte;
  if (r.after && typeof r.after.text === 'string') {
    sortie += `\n\n${gras('### ' + r.after.title)}\n${r.after.text}`;
  }
  return sortie;
}

// Ce que le backend a dit de ses crochets sur stderr (« déclenché », « tu »).
function crochets(backend) {
  const lignes = backend.stderr.join('').split('\n').filter((l) => l.startsWith('[crochet'));
  return lignes.length ? gris(lignes.slice(-3).map((l) => `  ${l}`).join('\n')) : '';
}

async function main() {
  if (sansService) {
    for (const k of Object.keys(process.env)) if (estVariableDeService(k)) delete process.env[k];
  }
  const service = adressesDuService();
  titre(sansService ? 'Sans service d’embarquement (plein texte seul)'
    : `Avec le service d’embarquement (${service ? service.adresses.join(',') : 'aucune variable posée'})`);
  console.log(`dépôt   : ${depot}`);
  console.log(`binaire : ${binaryPath()}`);

  const tmp = fs.mkdtempSync(path.join(os.tmpdir(), 'rag3weaver-demo-'));
  const manifest = prepareManifest(path.join(templatesDir(), 'backends', 'code', 'backend.json'), {
    name: 'demo',
    database: path.join(tmp, 'demo.rag3db'),
    vector_extension: vectorExtensionPath(),
    workspace: { root: depot, commands: 'off', index: 'code' },
    tools: { run_command: undefined },
  });
  const manifestPath = path.join(tmp, 'backend.json');
  fs.writeFileSync(manifestPath, JSON.stringify(manifest, null, 2));

  const t0 = Date.now();
  const backend = await Backend.open(manifestPath);
  console.log(`backend ouvert en ${Date.now() - t0} ms`);

  // ── describe : ce que le backend sait faire, et ce qu'il avertit.
  titre('describe — les outils du backend');
  const d = await backend.describe();
  for (const t of d.tools) {
    const premiere = String(t.description || '').split(/[.—]/)[0].trim();
    console.log(`  ${gras(t.name.padEnd(12))} ${premiere}`);
  }
  console.log(`\n  plein texte : ${d.fts} · embarquements : ${d.embeddings.model} (${d.embeddings.dimensions} dim.)`);
  if (Array.isArray(d.warnings) && d.warnings.length) {
    console.log(`\n  ${gras('avertissement')} : ${d.warnings.join('\n  ')}`);
  }

  // ── Avant tout index : la recherche balaie les fichiers (mots exacts).
  titre(`search_code avant l’index — « ${symbole} » par balayage des fichiers`);
  console.log(rendu(await backend.call('search_code', { query: symbole, options: {} }), 25));

  // ── L'index, en fond ; on attend qu'il rende le verrou.
  titre('index — construire l’index du dépôt');
  const t1 = Date.now();
  const recu = await backend.call('index', { confirm: true });
  console.log(rendu(recu, 8));
  const texteFini = (e) => Object.entries(e).filter(([k]) => k !== 'warnings' && k !== 'busy')
    .every(([, v]) => v.text === 'ready' || v.text === 'not_declared');
  let etat = await backend.indexState();
  let derniere = '';
  while (etat.busy || !texteFini(etat)) {
    await dormir(1000);
    etat = await backend.indexState();
    const ligne = Object.entries(etat).filter(([k]) => k !== 'warnings' && k !== 'busy')
      .map(([k, v]) => `${k} ${v.text} · ${vecteurs(v)}`).join(' | ');
    if (ligne && ligne !== derniere) { console.log(gris(`  ${ligne}`)); derniere = ligne; }
  }
  // L'indexation rend le verrou avant que les vecteurs soient tous là : on
  // laisse la dette se payer (au plus trois minutes), tant qu'une entité
  // dit ses vecteurs en cours.
  for (let i = 0; i < 180; i++) {
    const entites = Object.entries(etat).filter(([k]) => k !== 'warnings' && k !== 'busy');
    const enCours = entites.some(([, v]) => v.vectors === 'in_progress');
    if (!enCours) break;
    await dormir(1000);
    etat = await backend.indexState();
    if (i % 10 === 9) console.log(gris(`  ${entites.map(([k, v]) => `${k} ${vecteurs(v)}`).join(' · ')}`));
  }
  console.log(`\n  indexé en ${((Date.now() - t1) / 1000).toFixed(1)} s`);
  for (const [k, v] of Object.entries(etat)) {
    if (k === 'warnings' || k === 'busy') continue;
    console.log(`  ${gras(k.padEnd(8))} texte ${v.text} · ${vecteurs(v)} · relations ${v.relations}`);
  }
  if (Array.isArray(etat.warnings) && etat.warnings.length) {
    console.log(`\n  ${gras('avertissement')} : ${etat.warnings[0]}`);
  }
  console.log(`\n  ${gras('embarqueur')} : ${await embarqueur(d.embeddings)}`);

  // ── La recherche hybride : une question en langue naturelle.
  titre(`search_code — « ${question} »`);
  console.log(rendu(await backend.call('search_code', { query: question, options: {} }), 160));
  console.log(crochets(backend));

  // ── Les usages d'un symbole, puis ce qu'une modification toucherait.
  titre(`usages — qui se sert de « ${symbole} » ?`);
  console.log(rendu(await backend.call('usages', { name: symbole }), 40));

  titre(`impact — si je modifie « ${symbole} », qu’est-ce qui bouge ?`);
  console.log(rendu(await backend.call('impact', { name: symbole, depth: 2 }), 50));

  const exit = await backend.shutdown();
  console.log(`\n${gris(`arrêt propre (code ${exit.code}) · base : ${tmp}`)}`);
  fs.rmSync(tmp, { recursive: true, force: true });
}

main().catch((e) => {
  console.error(`\n${gras('ÉCHEC')} : ${e.message || e}`);
  process.exit(1);
});
