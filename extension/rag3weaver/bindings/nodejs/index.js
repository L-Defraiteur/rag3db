'use strict';
// rag3weaver pour Node : lance le binaire natif `rag3weaver-backend` et lui
// parle en lignes JSON sur stdin/stdout. Une seule API : celle du backend
// (describe, call, journal, journal_read, index_state, shutdown).
//
// Le binaire vient, dans cet ordre : de RAG3WEAVER_BACKEND (un chemin), du
// paquet de la plateforme que npm a installé (rag3weaver-linux-x64-gnu, …),
// ou d'un bâti local (dist/<plateforme>/ à la racine du dépôt).
const { spawn } = require('node:child_process');
const fs = require('node:fs');
const path = require('node:path');
const readline = require('node:readline');

const { platform, arch } = process;

function platformKey() {
  if (platform === 'linux') return `linux-${arch}-gnu`;
  if (platform === 'darwin') return `darwin-${arch}`;
  if (platform === 'win32') return `windows-${arch}`;
  return `${platform}-${arch}`;
}

const EXE = platform === 'win32' ? 'rag3weaver-backend.exe' : 'rag3weaver-backend';

/** Le dossier qui porte le binaire et l'extension vecteur, ou une erreur qui dit où on a cherché. */
function platformDir() {
  const key = platformKey();
  const tried = [];
  if (process.env.RAG3WEAVER_BACKEND) {
    const p = path.resolve(process.env.RAG3WEAVER_BACKEND);
    if (fs.existsSync(p)) return path.dirname(p);
    tried.push(`RAG3WEAVER_BACKEND=${p}`);
  }
  const pkg = `rag3weaver-${key}`;
  try {
    const dir = path.dirname(require.resolve(`${pkg}/package.json`));
    if (fs.existsSync(path.join(dir, EXE))) return dir;
    tried.push(`${pkg} (installed, but no ${EXE} inside)`);
  } catch (e) {
    tried.push(`${pkg}: ${String(e.message).split('\n')[0]}`);
  }
  for (const dir of [path.join(__dirname, 'npm', key), path.join(__dirname, '..', '..', '..', '..', 'dist', key)]) {
    if (fs.existsSync(path.join(dir, EXE))) return dir;
    tried.push(dir);
  }
  throw new Error(
    `rag3weaver: no backend binary for ${platform}-${arch}.\n` +
    `Prebuilt: Linux x64 (glibc >= 2.28). Elsewhere: build it with tools/build-images in https://github.com/L-Defraiteur/rag3db.\n` +
    tried.map((t) => '  ' + t).join('\n'),
  );
}

function binaryPath() { return path.join(platformDir(), EXE); }
function vectorExtensionPath() {
  const dir = platformDir();
  const name = fs.readdirSync(dir).find((f) => f.endsWith('.rag3db_extension'));
  if (!name) throw new Error(`rag3weaver: no vector extension next to ${path.join(dir, EXE)}`);
  return path.join(dir, name);
}

/** Les gabarits livrés avec le paquet (backends/code, tools), ou ceux du dépôt en développement. */
function templatesDir() {
  for (const d of [path.join(__dirname, 'templates'), path.join(__dirname, '..', '..', 'templates')]) {
    if (fs.existsSync(path.join(d, 'tools'))) return d;
  }
  throw new Error('rag3weaver: templates not found');
}

/**
 * Un manifeste prêt à écrire n'importe où : les chemins de graphes (relatifs
 * au gabarit) deviennent absolus, et `overrides` est fondu dedans.
 */
function prepareManifest(templatePath, overrides = {}) {
  const base = path.dirname(templatePath);
  const m = JSON.parse(fs.readFileSync(templatePath, 'utf8'));
  const absolutize = (node) => {
    if (Array.isArray(node)) return node.map(absolutize);
    if (node && typeof node === 'object') {
      for (const [k, v] of Object.entries(node)) {
        node[k] = k === 'graph' && typeof v === 'string' ? path.resolve(base, v) : absolutize(v);
      }
    }
    return node;
  };
  absolutize(m);
  const manifest = deepMerge(m, overrides);
  // Le bac à sable des commandes est Landlock, Linux seulement : ailleurs,
  // le backend refuse de démarrer tant que le manifeste ne dit pas
  // `"sandbox": {"mode": "off"}` en le sachant. Le paquet le dit pour vous,
  // sauf si vous l'avez écrit vous-même — et les commandes restent fermées
  // tant que `workspace.commands` ne les ouvre pas.
  if (process.platform !== 'linux' && manifest.workspace && manifest.workspace.sandbox === undefined) {
    manifest.workspace.sandbox = { mode: 'off' };
  }
  return manifest;
}

function deepMerge(a, b) {
  for (const [k, v] of Object.entries(b)) {
    if (v === undefined) delete a[k];
    else if (v && typeof v === 'object' && !Array.isArray(v) && a[k] && typeof a[k] === 'object') deepMerge(a[k], v);
    else a[k] = v;
  }
  return a;
}

/** Le backend lancé sur un manifeste. Une requête à la fois : le binaire répond dans l'ordre. */
class Backend {
  constructor(child, manifestPath) {
    this.child = child;
    this.manifestPath = manifestPath;
    this.pending = [];
    this.stderr = [];
    this.exited = null;
    child.stderr.setEncoding('utf8');
    child.stderr.on('data', (d) => { this.stderr.push(d); if (this.stderr.length > 50) this.stderr.shift(); });
    const lines = readline.createInterface({ input: child.stdout });
    lines.on('line', (line) => {
      const next = this.pending.shift();
      if (!next) return;
      let reply;
      try { reply = JSON.parse(line); } catch (e) { next.reject(new Error(`rag3weaver: unreadable reply: ${line}`)); return; }
      next.resolve(reply);
    });
    child.on('exit', (code, signal) => {
      this.exited = { code, signal };
      const err = new Error(`rag3weaver-backend exited (code ${code}, signal ${signal})${this.stderrText()}`);
      for (const p of this.pending.splice(0)) p.reject(err);
    });
  }

  stderrText() { const t = this.stderr.join('').trim(); return t ? `\n${t}` : ''; }

  /** Lance le binaire sur `manifestPath` et attend qu'il réponde à `describe`. */
  static async open(manifestPath, { binary, env } = {}) {
    const bin = binary || binaryPath();
    const child = spawn(bin, [manifestPath], { stdio: ['pipe', 'pipe', 'pipe'], env: { ...process.env, ...env } });
    const backend = new Backend(child, manifestPath);
    try {
      await backend.request({ op: 'describe' });
    } catch (e) {
      throw new Error(`rag3weaver: backend did not start on ${manifestPath}: ${e.message}`);
    }
    return backend;
  }

  request(message) {
    if (this.exited) return Promise.reject(new Error(`rag3weaver-backend already exited${this.stderrText()}`));
    return new Promise((resolve, reject) => {
      this.pending.push({ resolve, reject });
      this.child.stdin.write(JSON.stringify(message) + '\n');
    });
  }

  async op(message) {
    const reply = await this.request(message);
    if (!reply.ok) {
      const err = new Error(reply.error || 'rag3weaver: call failed');
      err.mustReopen = Boolean(reply.mustReopen);
      throw err;
    }
    return reply.result;
  }

  describe() { return this.op({ op: 'describe' }); }
  call(name, args = {}) { return this.op({ op: 'call', name, arguments: args }); }
  journal(events) { return this.op({ op: 'journal', events }); }
  journalRead(conversation, sinceMs = 0) { return this.op({ op: 'journal_read', conversation, since_ms: sinceMs }); }
  indexState() { return this.op({ op: 'index_state' }); }

  /** Ferme la base proprement (le point de reprise est écrit) et attend la fin du processus. */
  async shutdown() {
    if (this.exited) return this.exited;
    const done = new Promise((resolve) => this.child.once('exit', (code, signal) => resolve({ code, signal })));
    try { await this.op({ op: 'shutdown' }); } catch (e) { /* le processus s'arrête de toute façon */ }
    this.child.stdin.end();
    return done;
  }
}

module.exports = { Backend, binaryPath, vectorExtensionPath, templatesDir, prepareManifest, platformKey };
