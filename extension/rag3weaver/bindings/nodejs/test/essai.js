'use strict';
// L'épreuve du paquet : un dossier vide, trois fichiers, une recherche.
// Sans service d'embarquement : le backend de code en mots seuls (pas de
// `workspace.index`), donc search_code balaie les fichiers et grep_files
// cherche un motif. Le binaire vient de RAG3WEAVER_BACKEND, du paquet de la
// plateforme, ou de dist/<plateforme>/.
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const { Backend, binaryPath, vectorExtensionPath, templatesDir, prepareManifest } = require('..');

async function main() {
  const tmp = fs.mkdtempSync(path.join(os.tmpdir(), 'rag3weaver-essai-'));
  const ws = path.join(tmp, 'ws');
  fs.mkdirSync(ws);
  fs.writeFileSync(path.join(ws, 'main.rs'), 'fn main() {\n    println!("bonjour depuis rag3weaver");\n}\n');
  fs.writeFileSync(path.join(ws, 'README.md'), '# Essai\n\nTrois fichiers, une recherche : le mot cherché est « bonjour ».\n');
  fs.writeFileSync(path.join(ws, 'notes.txt'), 'rien à voir ici\n');

  const manifest = prepareManifest(path.join(templatesDir(), 'backends', 'code', 'backend.json'), {
    name: 'essai',
    database: path.join(tmp, 'essai.rag3db'),
    vector_extension: vectorExtensionPath(),
    // Sans commandes ni bac à sable (le test tourne partout) : la porte fermée
    // exige que l'outil run_command ne soit pas déclaré. Le schéma de code
    // s'enregistre par `index: "code"` ; en mots seuls (`index_signals` bm25),
    // le backend démarre sans service d'embarquement.
    workspace: {
      root: ws, commands: 'off', sandbox: { mode: 'off' },
      index: 'code', index_signals: { File: ['bm25'], Scope: ['bm25'] },
    },
    tools: { run_command: undefined },
  });
  const manifestPath = path.join(tmp, 'backend.json');
  fs.writeFileSync(manifestPath, JSON.stringify(manifest, null, 1));

  console.log(`binaire : ${binaryPath()}`);
  const t0 = Date.now();
  const backend = await Backend.open(manifestPath);
  console.log(`démarré en ${Date.now() - t0} ms`);
  const d = await backend.describe();
  const tools = d.tools.map((t) => t.name);
  console.log(`outils : ${tools.join(', ')}`);

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

  const search = await backend.call('search_code', { query: 'bonjour' });
  const s = JSON.stringify(search);
  if (!s.includes('main.rs') && !s.includes('README.md')) {
    throw new Error(`search_code "bonjour" avant tout index : attendu le balayage des fichiers : ${s.slice(0, 400)}`);
  }
  console.log('search_code : avant tout index, le balayage trouve le mot');

  // Indexer les trois fichiers (plein texte seul), puis chercher dans l'index.
  const recu = await backend.call('index', { confirm: true });
  console.log(`index : ${JSON.stringify(recu).slice(0, 160)}`);
  const etat = await backend.indexState();
  console.log(`état de l'index : ${JSON.stringify(etat).slice(0, 200)}`);
  const apres = await backend.call('search_code', { query: 'bonjour' });
  const a2 = JSON.stringify(apres);
  if (!a2.includes('main.rs') && !a2.includes('README.md')) {
    throw new Error(`search_code "bonjour" après l'index : ${a2.slice(0, 400)}`);
  }
  console.log('search_code : après l index, le mot est trouvé');

  const exit = await backend.shutdown();
  console.log(`arrêt propre : code ${exit.code}`);
  fs.rmSync(tmp, { recursive: true, force: true });
  console.log('OK');
}

main().catch((e) => { console.error(`ÉCHEC : ${e.message}`); process.exit(1); });
