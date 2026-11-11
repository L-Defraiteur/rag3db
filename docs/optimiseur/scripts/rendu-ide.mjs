// Rend une page de l'IDE d'un hôte dans Chrome sans tête par CDP : /ide/entrer (cookie), puis la page,
// lit le texte rendu, le nombre d'éléments, quelques balises et les erreurs de console.
// Usage : node rendu-ide.mjs <port> <chemin de la page, ex. /ide/ ou /ide/code/<id>/chat> ; JETON et SCRATCH dans l'env.
import { spawn } from 'node:child_process';
import { mkdtempSync, rmSync } from 'node:fs';
const [port, page] = process.argv.slice(2);
const base = `http://127.0.0.1:${port}`;
const dir = mkdtempSync(process.env.SCRATCH + '/cdp-');
const chrome = spawn('google-chrome-stable', ['--headless=new', '--disable-extensions', '--disable-gpu', '--no-first-run',
  `--user-data-dir=${dir}`, '--remote-debugging-port=9339', 'about:blank'], { stdio: 'ignore' });
const attendre = ms => new Promise(r => setTimeout(r, ms));
let cible;
for (let i = 0; i < 50 && !cible; i++) {
  await attendre(200);
  try { cible = (await (await fetch('http://127.0.0.1:9339/json')).json()).find(t => t.type === 'page'); } catch {}
}
const ws = new WebSocket(cible.webSocketDebuggerUrl);
await new Promise(r => ws.addEventListener('open', r));
let n = 0; const attente = new Map(); const erreurs = [];
ws.addEventListener('message', e => {
  const m = JSON.parse(e.data);
  if (m.id && attente.has(m.id)) { attente.get(m.id)(m); attente.delete(m.id); }
  if (m.method === 'Runtime.exceptionThrown') erreurs.push('exception : ' + JSON.stringify(m.params.exceptionDetails).slice(0, 400));
  if (m.method === 'Runtime.consoleAPICalled' && ['error', 'warning'].includes(m.params.type))
    erreurs.push(m.params.type + ' : ' + m.params.args.map(a => a.value ?? a.description).join(' ').slice(0, 400));
  if (m.method === 'Log.entryAdded' && ['error', 'warning'].includes(m.params.entry.level))
    erreurs.push('log ' + m.params.entry.level + ' : ' + m.params.entry.text.slice(0, 300) + ' ' + (m.params.entry.url || ''));
});
const cdp = (method, params = {}) => new Promise(r => { const id = ++n; attente.set(id, r); ws.send(JSON.stringify({ id, method, params })); });
const lire = async expression => (await cdp('Runtime.evaluate', { expression, returnByValue: true })).result.result.value;
await cdp('Runtime.enable'); await cdp('Log.enable'); await cdp('Page.enable');
await cdp('Page.navigate', { url: `${base}/ide/entrer?jeton=${process.env.JETON}` });
await attendre(4000);
console.log('après entrer :', await lire('location.pathname'));
erreurs.length = 0;
await cdp('Page.navigate', { url: `${base}${page}` });
await attendre(10000);
const t = await lire('document.getElementById("root")?.innerText ?? "(pas de #root)"');
console.log('--- texte de #root (début) ---\n' + t.slice(0, 1500));
console.log('--- éléments sous #root :', await lire('document.getElementById("root")?.querySelectorAll("*").length'));
console.log('--- balises :', await lire('["strong","li","code","button","nav","svg"].map(b => b + " " + document.querySelectorAll("#root " + b).length).join(", ")'));
// SELECTEURS="a|b|c" : le nombre d'éléments de chaque sélecteur ; CLIQUER=<texte d'un bouton> : un clic, puis
// SELECTEURS_APRES et FERMER=<texte d'un bouton> (on ne tape rien, on n'enregistre rien).
const compter = async liste => { for (const sel of liste.split('|').filter(Boolean))
  console.log(`    ${sel} : ${await lire(`document.querySelectorAll(${JSON.stringify(sel)}).length`)}`); };
if (process.env.SELECTEURS) { console.log('--- sélecteurs :'); await compter(process.env.SELECTEURS); }
const clic = async texte => lire(`(() => { const b = [...document.querySelectorAll('button')].find(x => x.innerText.trim() === ${JSON.stringify(texte)}); if (!b) return 'absent'; b.click(); return 'cliqué'; })()`);
if (process.env.CLIQUER) {
  console.log(`--- clic « ${process.env.CLIQUER} » :`, await clic(process.env.CLIQUER));
  await attendre(4000);
  await compter(process.env.SELECTEURS_APRES || '');
  console.log('    texte de .cm-content (début) :', JSON.stringify(((await lire('document.querySelector(".edition .cm-content")?.innerText')) || '').slice(0, 80)));
  if (process.env.FERMER) { console.log(`--- clic « ${process.env.FERMER} » :`, await clic(process.env.FERMER)); await attendre(2500); await compter(process.env.SELECTEURS_FIN || ''); }
}
if (process.env.EXPR) console.log('--- expr :', JSON.stringify(await lire(process.env.EXPR)).slice(0, 3000));
console.log('--- erreurs/avertissements de console :', erreurs.length ? '\n' + erreurs.join('\n') : 'aucun');
ws.close(); chrome.kill('SIGTERM');
await attendre(500);
try { rmSync(dir, { recursive: true, force: true }); } catch {}
