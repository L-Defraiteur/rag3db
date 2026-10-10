'use strict';
// L'épreuve du paquet : un dossier vide, trois fichiers, une recherche.
// Sans service d'embarquement : le backend de code démarre en le disant
// (`warnings` dans describe et index_state), indexe en plein texte, et la
// recherche dense dit « signal is not available ». Le binaire vient de
// RAG3WEAVER_BACKEND, du paquet de la plateforme, ou de dist/<plateforme>/.
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const { Backend, binaryPath, vectorExtensionPath, templatesDir, prepareManifest } = require('..');

async function main() {
  const tmp = fs.mkdtempSync(path.join(os.tmpdir(), 'rag3weaver-essai-'));
  const ws = path.join(tmp, 'ws');
  fs.mkdirSync(ws);
  // Une bibliothèque importée : l'entité Library est hybride par défaut,
  // c'est elle qui prendrait des vecteurs factices s'il y en avait.
  fs.writeFileSync(path.join(ws, 'main.rs'), 'use serde::Serialize;\n\n#[derive(Serialize)]\nstruct Salut { mot: String }\n\nfn main() {\n    println!("bonjour depuis rag3weaver");\n}\n');
  fs.writeFileSync(path.join(ws, 'README.md'), '# Essai\n\nTrois fichiers, une recherche : le mot cherché est « bonjour ».\n');
  fs.writeFileSync(path.join(ws, 'notes.txt'), 'rien à voir ici\n');

  const manifest = prepareManifest(path.join(templatesDir(), 'backends', 'code', 'backend.json'), {
    name: 'essai',
    database: path.join(tmp, 'essai.rag3db'),
    vector_extension: vectorExtensionPath(),
    // Sans commandes ni bac à sable (le test tourne partout) : la porte fermée
    // exige que l'outil run_command ne soit pas déclaré. Le schéma de code
    // s'enregistre par `index: "code"` ; sans service d'embarquement, le
    // backend démarre en le disant (plein texte seul, vecteurs en dette).
    workspace: {
      root: ws, commands: 'off', sandbox: { mode: 'off' },
      // Les signaux par défaut (hybride) : sans service, le dense se replie
      // en le disant et les vecteurs restent en dette — c'est le témoin.
      index: 'code',
    },
    tools: { run_command: undefined },
  });
  const manifestPath = path.join(tmp, 'backend.json');
  fs.writeFileSync(manifestPath, JSON.stringify(manifest, null, 1));

  console.log(`binaire : ${binaryPath()}`);
  const t0 = Date.now();
  const backend = await Backend.open(manifestPath);
  backendEnCours = backend;
  console.log(`démarré en ${Date.now() - t0} ms`);
  const d = await backend.describe();
  const tools = d.tools.map((t) => t.name);
  console.log(`outils : ${tools.join(', ')}`);
  // Sans service, le backend le dit dans son reçu, pas seulement sur stderr.
  if (!Array.isArray(d.warnings) || !d.warnings.some((w) => w.includes("pas de service d'embarquement"))) {
    throw new Error(`describe sans l'avertissement « pas de service d'embarquement » : ${JSON.stringify(d.warnings)}`);
  }
  console.log(`describe : ${d.warnings[0].slice(0, 80)}…`);

  const files = await backend.call('list_files', {});
  const listed = JSON.stringify(files);
  for (const f of ['main.rs', 'README.md', 'notes.txt']) {
    if (!listed.includes(f)) throw new Error(`list_files ne voit pas ${f} : ${listed.slice(0, 300)}`);
  }
  console.log('list_files : les trois fichiers');

  const grep = await backend.call('grep_files', { pattern: 'bonjour' });
  const g = JSON.stringify(grep);
  if (!g.includes('main.rs') || !g.includes('README.md') || g.includes('notes.txt')) {
    throw new Error(`grep_files "bonjour" : attendu main.rs et README.md, pas notes.txt : ${g.slice(0, 400)}`);
  }
  console.log('grep_files : « bonjour » dans main.rs et README.md, pas dans notes.txt');

  const search = await backend.call('search_code', { query: 'bonjour', options: {} });
  const s = JSON.stringify(search);
  if (!s.includes('main.rs') && !s.includes('README.md')) {
    throw new Error(`search_code "bonjour" avant tout index : attendu le balayage des fichiers : ${s.slice(0, 400)}`);
  }
  console.log('search_code : avant tout index, le balayage trouve le mot');

  // Indexer les trois fichiers (plein texte seul), puis chercher dans l'index.
  const recu = await backend.call('index', { confirm: true });
  console.log(`index : ${JSON.stringify(recu).slice(0, 160)}`);
  // L'indexation tourne en fond : on attend qu'elle rende le verrou.
  let etat = await backend.indexState();
  for (let i = 0; i < 120 && etat.busy; i++) {
    await new Promise((r) => setTimeout(r, 500));
    etat = await backend.indexState();
  }
  console.log(`état de l'index : ${JSON.stringify(etat).slice(0, 200)}`);
  if (!Array.isArray(etat.warnings) || etat.warnings.length === 0) {
    throw new Error(`index_state sans warnings : ${JSON.stringify(etat).slice(0, 300)}`);
  }
  const apres = await backend.call('search_code', { query: 'bonjour', options: {} });
  const a2 = JSON.stringify(apres);
  if (!a2.includes('main.rs') && !a2.includes('README.md')) {
    throw new Error(`search_code "bonjour" après l'index : ${a2.slice(0, 400)}`);
  }
  console.log('search_code : après l index, le mot est trouvé');
  if (!a2.includes('not available')) {
    throw new Error(`la recherche hybride sans service doit dire que le dense n'est pas disponible : ${a2.slice(0, 600)}`);
  }
  console.log('search_code : le dense se replie en le disant (« not available »)');
  const vecteurs = Object.entries(etat).filter(([k]) => k !== 'warnings')
    .map(([k, v]) => `${k}: ${v.vectors === 'not_declared' ? 'sans vecteurs (non déclarés)' : `vecteurs ${v.vectors} (${v.vectors_percent}%)`}`);
  console.log(`dette : ${vecteurs.join(' · ')}`);

  const exit = await backend.shutdown();
  console.log(`arrêt propre : code ${exit.code}`);
  fs.rmSync(tmp, { recursive: true, force: true });
  console.log('OK');
}

let backendEnCours = null;
main().catch((e) => {
  // Ce que le backend a dit sur stderr : c'est là que vivent les causes
  // (« plein texte … index non ouvert », dix-huitième essai Windows).
  if (backendEnCours && backendEnCours.stderr && backendEnCours.stderr.length) {
    console.error('--- stderr du backend (fin) ---');
    console.error(backendEnCours.stderr.join('').split('\n').slice(-25).join('\n'));
  } console.error(`ÉCHEC : ${e.message}`); process.exit(1); });
