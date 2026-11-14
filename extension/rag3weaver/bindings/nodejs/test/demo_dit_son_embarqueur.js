'use strict';
// Le témoin de la démo (ticket 2026-11-14-sans-service-la-demo-nodejs-embarque-quand-meme) :
// demo.js joué contre un faux paquet `rag3weaver`, qui note l'environnement que
// le binaire recevrait, et contre un faux service qui répond à /sante.
//   1. la passe sans service ne laisse au binaire AUCUNE variable de service
//      (RAG3WEAVER_SERVICE_* et l'alias RAG3WEAVER_EMBED_SERVICE), et dit
//      « embarqueur : aucun » ;
//   2. la passe avec service lit la variable commune (RAG3WEAVER_SERVICE_EMBED)
//      comme le binaire, et nomme le modèle et l'adresse qui l'ont servi.
// Aucun binaire, aucun réseau hors de la boucle locale.
const assert = require('node:assert');
const fs = require('node:fs');
const http = require('node:http');
const os = require('node:os');
const path = require('node:path');
const { execFile } = require('node:child_process');

const DEMO = path.join(__dirname, '..', 'demo', 'demo.js');

// Le faux paquet : Backend.open écrit l'environnement de service qu'il voit.
const FAUX_PAQUET = `'use strict';
const fs = require('node:fs');
const path = require('node:path');
const vues = () => Object.fromEntries(Object.entries(process.env)
  .filter(([k]) => k.startsWith('RAG3WEAVER_SERVICE_') || k === 'RAG3WEAVER_EMBED_SERVICE'));
const etat = { File: { text: 'ready', vectors: 'ready', vectors_percent: 100, relations: 'ready' } };
module.exports = {
  binaryPath: () => '/faux/rag3weaver-backend',
  vectorExtensionPath: () => '/faux/vector',
  templatesDir: () => __dirname,
  prepareManifest: (_p, o) => o,
  Backend: { open: async () => {
    fs.writeFileSync(process.env.TEMOIN_ENV, JSON.stringify(vues()));
    return {
      stderr: [],
      describe: async () => ({ tools: [], fts: 'lucivy', embeddings: { model: 'granite-278m', dimensions: 768 }, warnings: [] }),
      call: async () => ({ result: 'ok' }),
      indexState: async () => etat,
      shutdown: async () => ({ code: 0 }),
    };
  } },
};
`;

function installer() {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'rag3weaver-temoin-demo-'));
  fs.mkdirSync(path.join(dir, 'node_modules', 'rag3weaver'), { recursive: true });
  fs.writeFileSync(path.join(dir, 'node_modules', 'rag3weaver', 'index.js'), FAUX_PAQUET);
  fs.copyFileSync(DEMO, path.join(dir, 'demo.js')); // comme demo.sh : demo.js à côté du paquet
  fs.mkdirSync(path.join(dir, 'depot'));
  return dir;
}

function jouer(dir, env, ...args) {
  const temoinEnv = path.join(dir, `env-${args.join('-') || 'avec'}.json`);
  const propre = Object.fromEntries(Object.entries(process.env)
    .filter(([k]) => !k.startsWith('RAG3WEAVER_SERVICE_') && k !== 'RAG3WEAVER_EMBED_SERVICE'));
  return new Promise((resolve) => {
    execFile('node', ['demo.js', path.join(dir, 'depot'), ...args],
      { cwd: dir, env: { ...propre, ...env, TEMOIN_ENV: temoinEnv }, timeout: 30000 },
      (err, stdout, stderr) => {
        const sortie = (stdout + stderr).replace(/\x1b\[[0-9;]*m/g, '');
        const vu = fs.existsSync(temoinEnv) ? JSON.parse(fs.readFileSync(temoinEnv, 'utf8')) : null;
        resolve({ code: err ? err.code : 0, sortie, vu });
      });
  });
}

async function main() {
  // Un faux fournisseur : son /sante dit qui il est, comme rag3weaver-embeddings.
  const service = http.createServer((req, res) => {
    res.setHeader('Content-Type', 'application/json');
    res.end(JSON.stringify(req.url === '/sante'
      ? { modele: 'granite-278m', dim: 768, service: 'rag3weaver-embeddings', adresse: '127.0.0.1:7878' } : {}));
  });
  await new Promise((r) => service.listen(0, '127.0.0.1', r));
  const adresse = `127.0.0.1:${service.address().port}`;
  const dir = installer();
  let rouges = 0;
  const temoin = (nom, f) => {
    try { f(); console.log(`ok    ${nom}`); } catch (e) { rouges++; console.log(`ROUGE ${nom}\n      ${e.message.split('\n').join('\n      ')}`); }
  };
  try {
    const sans = await jouer(dir, {
      RAG3WEAVER_SERVICE_EMBED: adresse, RAG3WEAVER_EMBED_SERVICE: adresse, RAG3WEAVER_SERVICE_SPARSE: adresse,
    }, 'sans-service');
    temoin('sans service : le binaire ne reçoit aucune variable de service', () => {
      assert.strictEqual(sans.code, 0, sans.sortie);
      assert.deepStrictEqual(sans.vu, {}, `variables restées : ${JSON.stringify(sans.vu)}`);
    });
    temoin('sans service : la démo dit « embarqueur : aucun »', () => {
      assert.match(sans.sortie, /embarqueur : aucun/, sans.sortie);
    });

    const avec = await jouer(dir, { RAG3WEAVER_SERVICE_EMBED: adresse });
    temoin('avec service : l’en-tête nomme l’adresse de RAG3WEAVER_SERVICE_EMBED', () => {
      assert.strictEqual(avec.code, 0, avec.sortie);
      assert.match(avec.sortie, new RegExp(`Avec le service d’embarquement \\(${adresse}\\)`), avec.sortie);
    });
    temoin('avec service : la démo nomme le modèle et l’adresse qui l’a servi', () => {
      assert.match(avec.sortie, new RegExp(`embarqueur : granite-278m servi par ${adresse}`), avec.sortie);
    });
  } finally {
    service.close();
    fs.rmSync(dir, { recursive: true, force: true });
  }
  console.log(rouges ? `ROUGE (${rouges})` : 'VERT');
  process.exit(rouges ? 1 : 0);
}

main().catch((e) => { console.error(e); process.exit(1); });
