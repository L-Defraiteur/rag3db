#!/usr/bin/env node
'use strict';
// L'unité systemd utilisateur de l'hôte rag3weaver, en option du paquet
// (page docs/optimiseur/11-octobre-2026-15h46/01-l-unite-systemd-de-l-hote.md).
// Jamais installée par défaut : seulement par cette commande.
//
//   rag3weaver-unit install   [--name <unité>] [--env CLE=VALEUR]…
//   rag3weaver-unit uninstall [--name <unité>]
//   rag3weaver-unit status    [--name <unité>]
//
// L'unité lance `rag3weaver-backend daemon` (jamais --address : le refus d'un
// second hôte doit jouer), arrête par SIGTERM sans délai (l'arrêt propre de
// l'hôte), relance sur échec sauf le code 1 (le refus d'un second hôte et les
// erreurs de configuration), et écrit dans le même journal que l'hôte lancé
// par un client.
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const { execFileSync, spawnSync } = require('node:child_process');

// Les variables que l'hôte lancé par un client hérite de lui, et qu'une unité
// n'a pas : on fige celles qui sont posées au moment de l'installation.
const VARIABLES = [
  'RAG3WEAVER_DAEMON_DIR', 'RAG3WEAVER_DAEMON_ADDRESS',
  'RAG3WEAVER_EMBED_SERVICE', 'RAG3WEAVER_SERVICE_EMBED',
  'RAG3DB_ROOT', 'LD_LIBRARY_PATH',
];

function arguments_() {
  const a = process.argv.slice(2);
  const verbe = a.shift();
  let nom = 'rag3weaver-daemon';
  const env = {};
  while (a.length) {
    const x = a.shift();
    if (x === '--name') nom = a.shift();
    else if (x === '--env') { const kv = a.shift() || ''; const i = kv.indexOf('='); if (i > 0) env[kv.slice(0, i)] = kv.slice(i + 1); }
    else throw new Error(`argument inconnu : ${x}`);
  }
  if (!/^[A-Za-z0-9_.@-]+$/.test(nom)) throw new Error(`nom d'unité illisible : ${nom}`);
  return { verbe, nom, env };
}

const accueil = os.homedir();
const configuration = process.env.XDG_CONFIG_HOME || path.join(accueil, '.config');
const dossierUnites = path.join(configuration, 'systemd', 'user');

// Les mêmes chemins que src/daemon_lifecycle.rs : daemon.json et le journal.
function dossierDemon(env) { return env.RAG3WEAVER_DAEMON_DIR || path.join(accueil, '.local', 'share', 'rag3weaver'); }
function journalDemon(env) {
  return env.RAG3WEAVER_DAEMON_DIR
    ? path.join(env.RAG3WEAVER_DAEMON_DIR, 'daemon.log')
    : path.join(accueil, '.local', 'state', 'rag3weaver', 'daemon.log');
}

function systemctl(...args) {
  const r = spawnSync('systemctl', ['--user', ...args], { encoding: 'utf8' });
  return { code: r.status, sortie: (r.stdout || '').trim(), erreurs: (r.stderr || '').trim() };
}

function cite(v) { return /[\s"\\]/.test(v) ? `"${v.replace(/\\/g, '\\\\').replace(/"/g, '\\"')}"` : v; }

function texteUnite(nom, binaire, fichierEnv, journal) {
  return `# Écrit par rag3weaver-unit (paquet rag3weaver). Retirer : rag3weaver-unit uninstall --name ${nom}
[Unit]
Description=rag3weaver — l'hôte des backends (démon de l'agent de code)
Documentation=https://github.com/L-Defraiteur/rag3db

[Service]
Type=simple
EnvironmentFile=-${fichierEnv}
ExecStart=${cite(binaire)} daemon
# L'arrêt propre de l'hôte : l'appel en cours finit, les bases sont fermées,
# daemon.json retiré. Jamais de SIGKILL après un délai (comme PostgreSQL).
KillSignal=SIGTERM
KillMode=mixed
TimeoutStopSec=infinity
# Relancé s'il meurt d'un signal ou panique ; pas sur le code 1, qui est le
# refus d'un second hôte (un hôte lancé par un client tourne déjà) ou une
# erreur de configuration : relancer bouclerait.
Restart=on-failure
RestartPreventExitStatus=1
RestartSec=5
# Le même journal que l'hôte lancé par un client.
StandardOutput=append:${journal}
StandardError=append:${journal}

[Install]
WantedBy=default.target
`;
}

function lireFiche(env) {
  try { return JSON.parse(fs.readFileSync(path.join(dossierDemon(env), 'daemon.json'), 'utf8')); } catch { return null; }
}

function mainPid(nom) {
  const r = systemctl('show', '-p', 'MainPID', '--value', `${nom}.service`);
  return Number(r.sortie) || 0;
}

async function attendre(cond, ms) {
  const fin = Date.now() + ms;
  while (Date.now() < fin) { const v = cond(); if (v) return v; await new Promise((r) => setTimeout(r, 200)); }
  return null;
}

async function installer(nom, envPlus) {
  const { binaryPath } = require('..');
  const binaire = binaryPath();
  const env = {};
  for (const k of VARIABLES) if (process.env[k]) env[k] = process.env[k];
  Object.assign(env, envPlus);
  const fichierEnv = path.join(configuration, 'rag3weaver', `${nom}.env`);
  const journal = journalDemon(env);
  fs.mkdirSync(path.dirname(fichierEnv), { recursive: true });
  fs.writeFileSync(fichierEnv, Object.entries(env).map(([k, v]) => `${k}=${cite(v)}`).join('\n') + '\n', { mode: 0o600 });
  fs.mkdirSync(path.dirname(journal), { recursive: true });
  fs.mkdirSync(dossierUnites, { recursive: true });
  const fichierUnite = path.join(dossierUnites, `${nom}.service`);
  fs.writeFileSync(fichierUnite, texteUnite(nom, binaire, fichierEnv, journal));
  for (const etape of [['daemon-reload'], ['enable', '--now', `${nom}.service`]]) {
    const r = systemctl(...etape);
    if (r.code !== 0) throw new Error(`systemctl --user ${etape.join(' ')} : ${r.erreurs || r.sortie}`);
  }
  console.log(`unité ${fichierUnite}`);
  console.log(`environnement ${fichierEnv} (${Object.keys(env).join(', ') || 'aucune variable'})`);
  console.log(`journal ${journal}`);
  // L'hôte est prêt quand daemon.json porte le pid de l'unité.
  const pret = await attendre(() => { const p = mainPid(nom); const f = lireFiche(env); return p && f && f.pid === p ? f : null; }, 60000);
  if (!pret) {
    const actif = systemctl('is-active', `${nom}.service`).sortie;
    console.log(`l'hôte de l'unité n'a pas écrit daemon.json en 60 s (unité : ${actif}) — voir le journal ; un hôte lancé par un client tourne peut-être déjà (refus d'un second hôte)`);
    process.exitCode = 3;
    return;
  }
  console.log(`hôte prêt : pid ${pret.pid}, ${pret.address}`);
}

function desinstaller(nom) {
  const fichierUnite = path.join(dossierUnites, `${nom}.service`);
  const fichierEnv = path.join(configuration, 'rag3weaver', `${nom}.env`);
  // disable --now : SIGTERM, l'arrêt propre ; on attend qu'il soit fini.
  systemctl('disable', '--now', `${nom}.service`);
  for (const f of [fichierUnite, fichierEnv]) { try { fs.unlinkSync(f); console.log(`retiré ${f}`); } catch { /* absent */ } }
  systemctl('daemon-reload');
  systemctl('reset-failed', `${nom}.service`);
  console.log(`unité ${nom} désinstallée ; le prochain client relancera un hôte à la main`);
}

function etat(nom, envPlus) {
  const env = { ...process.env, ...envPlus };
  const fichierUnite = path.join(dossierUnites, `${nom}.service`);
  const installee = fs.existsSync(fichierUnite);
  console.log(`unité ${nom} : ${installee ? 'installée' : 'non installée'}`);
  if (installee) {
    console.log(`  activée : ${systemctl('is-enabled', `${nom}.service`).sortie} · état : ${systemctl('is-active', `${nom}.service`).sortie} · MainPID : ${mainPid(nom) || '—'}`);
  }
  const f = lireFiche(env);
  console.log(f ? `daemon.json : pid ${f.pid}, ${f.address}` : 'daemon.json : absent (aucun hôte, ou arrêté proprement)');
}

(async () => {
  try {
    if (process.platform !== 'linux') throw new Error('l\'unité systemd est pour Linux seulement');
    const { verbe, nom, env } = arguments_();
    if (verbe === 'install') await installer(nom, env);
    else if (verbe === 'uninstall') desinstaller(nom);
    else if (verbe === 'status') etat(nom, env);
    else { console.error('usage : rag3weaver-unit install|uninstall|status [--name <unité>] [--env CLE=VALEUR]…'); process.exitCode = 2; }
  } catch (e) {
    console.error(`rag3weaver-unit : ${e.message}`);
    process.exitCode = 1;
  }
})();
