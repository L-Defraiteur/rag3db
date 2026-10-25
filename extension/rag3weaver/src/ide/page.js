// ─── Le socle ──────────────────────────────────────────────────────────────
// La page n'est qu'une vue : chaque URL nomme ce qu'elle montre, tout vient
// des outils de l'hôte (/ide/call), et le flux (/ide/flux) la tient à jour.

const vue = document.getElementById("vue");
const fil = document.getElementById("fil");

function el(tag, attrs = {}, ...enfants) {
  const e = document.createElement(tag);
  for (const [k, v] of Object.entries(attrs)) {
    if (v === undefined || v === null) continue;
    if (k === "class") e.className = v;
    else if (k.startsWith("on") && typeof v === "function") e.addEventListener(k.slice(2), v);
    else e.setAttribute(k, v);
  }
  for (const c of enfants.flat()) e.append(c instanceof Node ? c : document.createTextNode(String(c)));
  return e;
}
const enc = (s) => s.split("/").map(encodeURIComponent).join("/");

let etat = null;
async function chargerEtat() {
  etat = await (await fetch("/ide/etat")).json();
  return etat;
}
function backend(id) {
  return (etat?.backends ?? []).find((b) => b.id === id);
}

// La réponse entière du graphe de l'outil ({ result, metadata, freshness,
// validation, … }) : pour une vue qui montre aussi ce que l'hôte dit à côté de
// la sortie (la recherche : sa méta, la fraîcheur de l'index).
async function appelerEntier(id, nom, args) {
  const b = backend(id);
  if (!b) throw new Error(`backend inconnu : ${id}`);
  const r = await fetch("/ide/call", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ manifest: b.manifest, name: nom, arguments: args }),
  });
  const v = await r.json();
  if (!v.ok) throw new Error(v.error ?? `${nom} : échec`);
  return v.result;
}

async function appeler(id, nom, args) {
  const r = await appelerEntier(id, nom, args);
  // `/ide/call` rend la réponse du graphe de l'outil ({ result, metadata,
  // validation, … }) : ce que la vue lit est sa sortie, `result` (le témoin des
  // deux fenêtres l'a montré, 18 octobre 2026 : `r.text` et `r.content` étaient
  // lus un cran trop haut).
  if (r && typeof r === "object" && "result" in r) return sortie(r.result);
  // Toute réponse d'un outil du backend porte `result` : ce repli ne devrait
  // jamais servir, et s'il sert, une forme inattendue ne passe pas en silence.
  console.warn(`${nom} : réponse de /ide/call sans « result », rendue telle quelle`, r);
  return r;
}

// Une lecture du catalogue (usages, impact, callees…) enveloppe sa sortie de
// sa ligne d'état — `{ status, result }`, `with_status` côté hôte — et un refus
// est `{ status, result: null }`. La vue lit la sortie ; la ligne d'état se
// montre en tête de la vue, le refus devient l'erreur affichée (vu par le témoin
// navigateur, 19 octobre 2026 : le symbole montrait « Définitions (0) »).
let lignesDEtat = new Set();
function sortie(s) {
  if (!s || typeof s !== "object" || Array.isArray(s)) return s;
  const cles = Object.keys(s);
  if (cles.length !== 2 || !cles.includes("status") || !cles.includes("result")) return s;
  if (s.result === null) throw new Error(s.status);
  lignesDEtat.add(s.status);
  return s.result;
}

function erreur(e) {
  vue.replaceChildren(el("p", { class: "alerte" }, String(e.message ?? e)));
}

function lienFichier(id, chemin, ligne) {
  return `/ide/code/${id}/fichier/${enc(chemin)}${ligne ? `#L${ligne}` : ""}`;
}
function lienSymbole(id, nom) {
  return `/ide/code/${id}/symbole/${encodeURIComponent(nom)}`;
}
function lienGraphe(id, nom) {
  return `/ide/code/${id}/graphe/${encodeURIComponent(nom)}`;
}

function filDAriane(id, chemin, dernierEstFichier) {
  const b = backend(id);
  const morceaux = [el("a", { href: `/ide/code/${id}/arbre/` }, b ? nomCourt(b.manifest) : id)];
  const parts = (chemin ?? "").split("/").filter(Boolean);
  parts.forEach((p, i) => {
    const sous = parts.slice(0, i + 1).join("/");
    const fichier = dernierEstFichier && i === parts.length - 1;
    morceaux.push(" / ", el("a", { href: fichier ? lienFichier(id, sous) : `/ide/code/${id}/arbre/${enc(sous)}/` }, p));
  });
  fil.replaceChildren(...morceaux);
  const form = document.getElementById("chercher");
  form.hidden = false;
  form.onsubmit = (ev) => {
    ev.preventDefault();
    const q = new FormData(form).get("q");
    if (q) location.href = `/ide/code/${id}/recherche?q=${encodeURIComponent(q)}`;
  };
}
const nomCourt = (manifeste) => manifeste.split("/").slice(-3, -1).join("/") || manifeste;

// ─── Les vues ─────────────────────────────────────────────────────────────

async function accueil() {
  fil.replaceChildren("les backends de l'hôte");
  const backends = etat.backends ?? [];
  if (!backends.length) {
    vue.replaceChildren(el("p", { class: "note" }, "L'hôte ne sert encore aucun backend : un client (MCP) l'ouvre à son premier appel."));
    return;
  }
  vue.replaceChildren(
    el("h1", {}, "Backends"),
    el("ul", { class: "liste" }, backends.map((b) =>
      el("li", {},
        el("a", { href: `/ide/code/${b.id}/arbre/` }, nomCourt(b.manifest)),
        el("a", { href: `/ide/code/${b.id}/dag/` }, "graphes"),
        el("a", { href: `/ide/code/${b.id}/reactions/` }, "réactions"),
        el("a", { href: `/ide/code/${b.id}/propositions/` }, "propositions"),
        b.tache ? el("span", { class: "pastille" }, `tâche ${b.tache.nom}`) : el("a", { href: `/ide/code/${b.id}/taches/` }, "tâches"),
        el("a", { href: `/ide/code/${b.id}/chat` }, "chat"),
        el("a", { href: `/ide/code/${b.id}/bilans` }, "bilans"),
        el("span", { class: "meta" }, b.manifest),
        el("span", { class: "meta" }, b.open ? "ouvert" : "fermé"),
        (b.template_drift ?? []).length ? el("span", { class: "meta" }, `${b.template_drift.length} gabarit(s) local(aux) différent(s) du livré`) : "",
      ))),
  );
}

async function arbre(id, dossier) {
  filDAriane(id, dossier, false);
  const prefixe = dossier ? (dossier.endsWith("/") ? dossier : dossier + "/") : "";
  const r = await appeler(id, "list_files", { path_prefix: prefixe, limit: 2000, format: "json" });
  const dossiers = new Map();
  const fichiers = [];
  for (const e of r.entries ?? []) {
    const reste = e.path.slice(prefixe.length);
    const [premier, ...suite] = reste.split("/");
    if (suite.length) dossiers.set(premier, (dossiers.get(premier) ?? 0) + 1);
    else fichiers.push(e);
  }
  const lignes = [
    ...[...dossiers.entries()].sort().map(([d, n]) =>
      el("li", {}, el("a", { href: `/ide/code/${id}/arbre/${enc(prefixe + d)}/` }, d + "/"), el("span", { class: "meta" }, `${n} fichier${n > 1 ? "s" : ""}`))),
    ...fichiers.sort((a, b) => a.path.localeCompare(b.path)).map((e) =>
      el("li", {},
        el("a", { href: lienFichier(id, e.path) }, e.path.slice(prefixe.length)),
        el("span", { class: "meta" }, e.lines != null ? `${e.lines} lignes` : ""),
        e.stale ? el("span", { class: "pastille" }, "modifié depuis l'index") : e.indexed === false ? el("span", { class: "pastille" }, "non indexé") : "",
      )),
  ];
  vue.replaceChildren(
    el("h1", {}, prefixe || "/"),
    r.total > r.returned ? el("p", { class: "note" }, `${r.returned} fichiers montrés sur ${r.total} sous ce préfixe.`) : "",
    el("ul", { class: "liste" }, lignes),
  );
}

// Les mots qui ne sont pas des symboles du dépôt (quelques langages courants).
const MOTS = new Set(("as async await break case catch class const continue crate default do dyn else enum export extends " +
  "false fn for from function if impl import in interface let loop match mod move mut new null pub ref return self Self " +
  "static struct super switch this throw trait true try type typeof unsafe use var void where while yield int bool char " +
  "float double long short unsigned signed auto template typename namespace using public private protected virtual override " +
  "str String Vec Option Some None Ok Err Result Box Arc usize u8 u16 u32 u64 i32 i64 f32 f64").split(" "));

function ligneCliquable(id, texte) {
  const frag = document.createDocumentFragment();
  let dernier = 0;
  for (const m of texte.matchAll(/[A-Za-z_][A-Za-z0-9_]*/g)) {
    if (m.index > dernier) frag.append(texte.slice(dernier, m.index));
    const mot = m[0];
    frag.append(MOTS.has(mot) || mot.length < 2 ? mot : el("a", { class: "id", href: lienSymbole(id, mot) }, mot));
    dernier = m.index + mot.length;
  }
  frag.append(texte.slice(dernier));
  return frag;
}

async function fichier(id, chemin) {
  edition = null; // la vue de lecture : aucune édition en cours
  filDAriane(id, chemin, true);
  const offset = Number(new URLSearchParams(location.search).get("depuis") ?? 1) || 1;
  const [r, plan] = await Promise.all([
    appeler(id, "read_file", { path: chemin, offset, limit: 2000, format: "json" }),
    // Le plan du fichier : l'outil outline, en json ({rows, total, provenance}).
    appeler(id, "outline", { path: chemin, format: "json" }).then((t) => (typeof t === "string" ? JSON.parse(t) : t)).catch((e) => ({ erreur: e.message })),
  ]);
  const corps = el("tbody");
  for (const brute of r.text.split("\n")) {
    const m = brute.match(/^(\d+)\| ?(.*)$/);
    if (!m) continue;
    const n = Number(m[1]);
    corps.append(el("tr", { id: `L${n}` },
      el("td", { class: "n" }, el("a", { href: `#L${n}` }, n)),
      el("td", {}, ligneCliquable(id, m[2]))));
  }
  const scopes = planDuFichier(id, plan);
  vue.replaceChildren(
    el("h1", {}, chemin),
    el("p", { class: "barre" }, el("button", { onclick: () => editer(id, chemin) }, "Modifier")),
    el("p", { class: "note" },
      `${r.total_lines} lignes`,
      r.stale === true ? el("span", { class: "alerte" }, " · modifié depuis l'index") : r.stale === null ? " · non indexé" : "",
      r.has_more ? el("span", {}, " · ", el("a", { href: `${location.pathname}?depuis=${r.offset + r.lines_read}` }, "la suite")) : ""),
    el("div", { class: "grille" },
      el("div", { class: "code" }, el("table", {}, corps)),
      el("aside", { id: "plan" }, el("h2", {}, "Dans ce fichier"), ...scopes),
    ),
  );
  vise();
}

// Le plan d'un fichier, tel que le rend outline en json : ses scopes dans
// l'ordre du fichier, chacun sous celui qui le contient (par les lignes, pas
// par le nom : deux scopes peuvent porter le même), vers sa ligne et vers son
// symbole ; la provenance que l'outil dit, et ce qui manque si `limit` a borné.
function planDuFichier(id, plan) {
  if (plan.erreur) return [el("p", { class: "note" }, `le plan : ${plan.erreur}`)];
  const lignes = plan.rows ?? [];
  const pile = [];
  const scopes = lignes.map((s) => {
    while (pile.length && pile[pile.length - 1] < s.start_line) pile.pop();
    const profondeur = pile.length;
    pile.push(s.end_line);
    return el("div", { class: "scope", style: `--profondeur: ${profondeur}`, title: s.signature ?? "", "data-ligne": s.start_line },
      el("a", { href: `#L${s.start_line}` }, s.name),
      " ", el("span", { class: "meta note" }, `${s.scope_type} ${s.start_line}–${s.end_line}`),
      " ", el("a", { href: lienSymbole(id, s.name), title: "usages, impact, appelés" }, "→"));
  });
  return [
    ...(scopes.length ? scopes : [el("p", { class: "note" }, "aucun scope")]),
    typeof plan.total === "number" && plan.total > lignes.length
      ? el("p", { class: "note", id: "plan-borne" }, `${lignes.length} scopes montrés sur ${plan.total}`) : "",
    plan.provenance ? el("p", { class: "note", id: "plan-provenance" }, plan.provenance) : "",
  ];
}

// ─── L'édition humaine (B1, page 05 §4) ──────────────────────────────────────
// La page n'est qu'une vue : le tampon vit le temps de l'édition, l'hôte fait foi.
// On lit le fichier exact (`format: "brut"`) et son empreinte ; l'enregistrement
// la rend (`expected_hash`) : si le fichier a changé depuis, rien n'est écrit,
// et la page montre la différence au lieu d'écraser.
let edition = null; // { id, chemin, hash, modifie, zone, avis }

async function editer(id, chemin) {
  const r = await appeler(id, "read_file", { path: chemin, format: "brut" });
  if (r.outside) {
    vue.prepend(el("p", { class: "alerte" }, "Ce fichier est hors du worktree déclaré : il se lit, il ne se modifie pas."));
    return;
  }
  const avis = el("span", { class: "note" }, "");
  const conteneur = el("div", { class: "edition", "aria-label": `contenu de ${chemin}` });
  const marquer = () => { if (edition) { edition.modifie = true; avis.textContent = "modifié, non enregistré"; } };
  edition = { id, chemin, hash: r.content_hash, modifie: false, zone: null, avis };
  vue.replaceChildren(
    el("h1", {}, chemin),
    el("p", { class: "barre" },
      el("button", { onclick: enregistrer }, "Enregistrer"),
      el("button", { onclick: quitter }, "Fermer l'édition"),
      avis),
    conteneur,
  );
  edition.zone = await zoneDEdition(conteneur, r.content, marquer, chemin);
  edition.zone.focus();
}

// ─── L'éditeur (B1, partie 2) ─────────────────────────────────────────────────
// CodeMirror, par le magasin de libs embarqué dans l'hôte (/ide/editeur/modules.js,
// rangé par scripts/ide_libs.sh) et le registre de l'aperçu : ni CDN, ni
// téléchargement à l'usage. La zone garde l'interface d'une textarea (value,
// focus, parentElement) : l'enregistrement, le conflit et la différence n'en
// savent rien. Ctrl-S (Cmd-S) enregistre ; le tampon n'est jamais remplacé par le
// disque sans le choix de l'utilisatrice.
let editeur = null; // la promesse des modules de CodeMirror, chargés une fois
function chargerScript(src) {
  return new Promise((ok, ko) => {
    const s = el("script", { src });
    s.onload = ok;
    s.onerror = () => ko(new Error(`${src} ne se charge pas`));
    document.head.append(s);
  });
}
function modulesDeLEditeur() {
  if (!editeur) {
    editeur = (async () => {
      if (!globalThis.__rag3weaver) await chargerScript("/ide/registre.js");
      await chargerScript("/ide/editeur/modules.js");
      const R = globalThis.__rag3weaver;
      const lib = (spec) => R.require(R.demarrage[spec]);
      return {
        view: lib("@codemirror/view"), state: lib("@codemirror/state"), commands: lib("@codemirror/commands"),
        search: lib("@codemirror/search"), language: lib("@codemirror/language"),
      };
    })();
  }
  return editeur;
}

async function zoneDEdition(conteneur, texte, marquer, chemin) {
  let cm;
  try {
    cm = await modulesDeLEditeur();
  } catch (err) {
    // Sans ses modules, l'édition reste possible : une textarea, et la raison dite.
    console.error(`l'éditeur ne se charge pas, une zone de texte le remplace : ${err.message ?? err}`);
    editeur = null;
    const zone = el("textarea", { spellcheck: "false", "aria-label": `contenu de ${chemin}` });
    zone.value = texte;
    zone.addEventListener("input", marquer);
    zone.addEventListener("keydown", (e) => {
      if ((e.ctrlKey || e.metaKey) && e.key === "s") { e.preventDefault(); enregistrer(); }
    });
    conteneur.append(zone);
    return zone;
  }
  const { EditorView, keymap, lineNumbers, highlightActiveLine, highlightActiveLineGutter, drawSelection } = cm.view;
  const { EditorState } = cm.state;
  const { defaultKeymap, history, historyKeymap, indentWithTab } = cm.commands;
  const { searchKeymap, highlightSelectionMatches } = cm.search;
  const { bracketMatching, indentOnInput } = cm.language;
  // Une écriture de la page (reprendre le disque) n'est pas une modification.
  let deLaPage = false;
  const vue = new EditorView({
    parent: conteneur,
    state: EditorState.create({
      doc: texte,
      extensions: [
        lineNumbers(), highlightActiveLineGutter(), highlightActiveLine(), drawSelection(),
        history(), bracketMatching(), indentOnInput(), highlightSelectionMatches(),
        keymap.of([
          { key: "Mod-s", preventDefault: true, run: () => { enregistrer(); return true; } },
          ...defaultKeymap, ...historyKeymap, ...searchKeymap, indentWithTab,
        ]),
        EditorView.updateListener.of((u) => { if (u.docChanged && !deLaPage) marquer(); }),
        EditorView.theme({ "&": { height: "70vh" }, ".cm-scroller": { fontFamily: "var(--mono)", fontSize: "13px" } }),
      ],
    }),
  });
  return {
    get value() { return vue.state.doc.toString(); },
    set value(t) {
      deLaPage = true;
      try { vue.dispatch({ changes: { from: 0, to: vue.state.doc.length, insert: t } }); } finally { deLaPage = false; }
    },
    focus() { vue.focus(); },
    get parentElement() { return conteneur; },
    vue,
  };
}

async function enregistrer() {
  const e = edition;
  if (!e) return;
  try {
    await appeler(e.id, "edit_file", { path: e.chemin, content: e.zone.value, expected_hash: e.hash });
  } catch (err) {
    if (String(err).includes("fichier changé depuis la lecture")) return conflit(e, String(err));
    e.avis.textContent = `refusé : ${err}`;
    return;
  }
  // Écrit : le tampon est l'état du disque, son empreinte se relit.
  const r = await appeler(e.id, "read_file", { path: e.chemin, format: "brut" });
  e.hash = r.content_hash;
  e.modifie = false;
  e.avis.textContent = "enregistré";
}

// Le fichier a changé depuis la lecture : la différence entre le disque et le
// tampon, puis le choix — reprendre le disque, ou écraser en le sachant.
async function conflit(e, raison) {
  const r = await appeler(e.id, "read_file", { path: e.chemin, format: "brut" });
  const choix = el("p", { class: "barre" },
    el("button", { onclick: () => { e.zone.value = r.content; e.hash = r.content_hash; e.modifie = false; e.avis.textContent = "repris du disque"; choix.remove(); diff.remove(); } }, "Reprendre la version du disque"),
    el("button", { onclick: () => { e.hash = r.content_hash; choix.remove(); diff.remove(); enregistrer(); } }, "Garder ma version (écrase la leur)"));
  const diff = el("div", { class: "diff", "aria-label": "différence disque → tampon" }, ...difference(r.content, e.zone.value));
  e.avis.textContent = raison;
  e.zone.parentElement.before(choix, diff);
}

// Une différence par lignes (plus longue sous-suite commune), bornée : au-delà,
// les deux versions entières se comparent à l'œil.
function difference(avant, apres) {
  const a = avant.split("\n"), b = apres.split("\n");
  if (a.length * b.length > 4e6) return [el("div", {}, "fichiers trop grands pour la différence ligne à ligne : reprenez le disque ou écrasez en connaissance de cause")];
  const l = Array.from({ length: a.length + 1 }, () => new Uint32Array(b.length + 1));
  for (let i = a.length - 1; i >= 0; i--) for (let j = b.length - 1; j >= 0; j--)
    l[i][j] = a[i] === b[j] ? l[i + 1][j + 1] + 1 : Math.max(l[i + 1][j], l[i][j + 1]);
  const lignes = [];
  let i = 0, j = 0;
  while (i < a.length || j < b.length) {
    if (i < a.length && j < b.length && a[i] === b[j]) { i++; j++; continue; }
    if (i < a.length && (j >= b.length || l[i + 1][j] >= l[i][j + 1])) lignes.push(el("div", { class: "moins" }, "- " + a[i++]));
    else lignes.push(el("div", { class: "plus" }, "+ " + b[j++]));
  }
  return lignes.length ? lignes : [el("div", {}, "aucune différence de lignes (fins de ligne ou espaces)")];
}

function quitter() {
  if (edition?.modifie && !confirm("Des modifications ne sont pas enregistrées : les abandonner ?")) return;
  edition = null;
  montrer();
}

function vise() {
  document.querySelectorAll("tr.vise").forEach((t) => t.classList.remove("vise"));
  const cible = location.hash && document.getElementById(location.hash.slice(1));
  if (cible) { cible.classList.add("vise"); cible.scrollIntoView({ block: "center" }); }
}
addEventListener("hashchange", vise);

function itemLien(id, it) {
  return el("li", {},
    el("a", { href: lienFichier(id, it.path, it.line) }, `${it.path}${it.line ? ":" + it.line : ""}`),
    el("a", { href: lienSymbole(id, it.title) }, it.title),
    el("span", { class: "meta" }, it.kind ?? ""));
}

async function symbole(id, nom) {
  filDAriane(id, "", false);
  fil.append(" · ", nom);
  const [u, imp, app] = await Promise.all([
    appeler(id, "usages", { name: nom, limit: 200, format: "json" }),
    appeler(id, "impact", { name: nom, format: "json" }).catch((e) => ({ erreur: e.message })),
    appeler(id, "callees", { name: nom, format: "json" }).catch((e) => ({ erreur: e.message })),
  ]);
  const parGenre = new Map();
  for (const x of u.usages ?? []) {
    if (!parGenre.has(x.usage)) parGenre.set(x.usage, []);
    parGenre.get(x.usage).push(x);
  }
  const voisins = (r, titre) => r.erreur
    ? [el("h2", {}, titre), el("p", { class: "note" }, r.erreur)]
    : [el("h2", {}, `${titre} (${(r.reached ?? []).length})`),
       el("ul", { class: "liste" }, (r.reached ?? []).map((it) => itemLien(id, it)))];
  vue.replaceChildren(
    el("h1", {}, nom, " ", el("a", { href: lienGraphe(id, nom), class: "pastille" }, "en graphe")),
    u.ambiguous ? el("p", { class: "note" }, "Nom défini plusieurs fois : les usages trouvés par le nom seul ne sont pas attribués.") : "",
    el("h2", {}, `Définitions (${(u.definitions ?? []).length})`),
    el("ul", { class: "liste" }, (u.definitions ?? []).map((d) => itemLien(id, d))),
    ...[...parGenre.entries()].flatMap(([genre, xs]) => [
      el("h2", {}, `${genre} (${xs.length})`),
      el("ul", { class: "liste" }, xs.map((x) => {
        const li = itemLien(id, x.user);
        if (x.guessed) li.append(el("span", { class: "pastille" }, "par le nom"));
        return li;
      })),
    ]),
    ...voisins(imp, "Ce qui en dépend (impact)"),
    ...voisins(app, "Ce qu'il appelle"),
  );
}

// **La recherche dit ce qu'elle sait** (pièce 3, docs/12-octobre-2026-recherche/06) :
// tout vient de la réponse de `search_code`, rien n'est relu dans son markdown.
// En tête, l'état de l'index (la ligne « index : » de la méta) et sa fraîcheur ;
// par résultat, le score, ce que chaque signal en a dit (score brut et rang) et
// l'extrait ; en pied, les avertissements (un signal en repli, un relecteur
// absent). « Reclasser » est dans l'URL, comme la requête : la page n'a pas
// d'état que l'hôte n'ait pas.
async function recherche(id) {
  filDAriane(id, "", false);
  const params = new URLSearchParams(location.search);
  const q = params.get("q") ?? "";
  // Le formulaire de l'en-tête repart sans `reclasser` : la relecture se
  // décide question par question (elle coûte une demi-seconde).
  const reclasser = params.get("reclasser") === "1";
  document.querySelector("#chercher input").value = q;
  fil.append(" · recherche");
  const options = { limit: 30 };
  if (reclasser) options.rerank = { candidates: 20 };
  const reponse = await appelerEntier(id, "search_code", { query: q, options });
  const r = sortie(reponse?.result);
  // La méta porte le nom du nœud qui la rend dans le graphe de search_code
  // (`render`, port `meta`, déclaré par le gabarit) : un renommage viderait
  // l'en-tête sans erreur ; le témoin navigateur, qui exige la ligne d'état,
  // le verrait.
  const meta = reponse?.metadata?.["render.meta"] ?? {};
  const avertis = meta.warnings ?? [];
  const etatIndex = avertis.find((w) => w.startsWith("index : "));
  const autres = avertis.filter((w) => !w.startsWith("index : "));
  const resultats = Array.isArray(r) ? r : [];
  const caseReclasser = el("input", { type: "checkbox", id: "reclasser", onchange: (e) => {
    const p = new URLSearchParams(location.search);
    if (e.target.checked) p.set("reclasser", "1"); else p.delete("reclasser");
    location.search = p.toString();
  } });
  caseReclasser.checked = reclasser;
  vue.replaceChildren(
    el("h1", {}, `« ${q} »`),
    el("p", { id: "recherche-etat", class: "note" }, etatRecherche(etatIndex, reponse?.freshness, meta, resultats.length)),
    el("p", { class: "note" }, el("label", {}, caseReclasser, " reclasser : un relecteur relit les 20 premiers (plus lent ; pour une question en phrases)")),
    ...resultats.map((x) => {
      const d = x.data ?? {};
      const extrait = x.chunk?.text ?? d.signature ?? "";
      return el("div", { class: "resultat" },
        el("a", { href: lienFichier(id, d.file_path ?? "", d.start_line) }, `${d.file_path ?? "?"}:${d.start_line ?? ""}`),
        " ", d.name ? el("a", { href: lienSymbole(id, d.name) }, d.name) : "",
        " ", el("span", { class: "meta note" }, `${d.scope_type ?? x.entity ?? ""} · ${x.signal ?? ""}`),
        " ", el("span", { class: "score", title: "le score de l'ordre rendu" }, `★ ${Number(x.score ?? 0).toFixed(3)}`),
        (x.signals ?? []).length
          ? el("div", { class: "signaux note" }, x.signals.map((s) => `${s.label} n°${s.rank} (${Number(s.score).toFixed(3)})`).join(" · "))
          : "",
        extrait ? el("pre", {}, extrait.length > 600 ? `${extrait.slice(0, 600)}…` : extrait) : "");
    }),
    autres.length ? el("ul", { class: "avertissements" }, autres.map((w) => el("li", {}, `⚠ ${w}`))) : "",
  );
}

// La ligne de tête : l'état de l'index tel que la méta le dit, sinon « à jour »,
// la fraîcheur (fichiers en retard, arbre modifié), et ce que la recherche a
// compté.
function etatRecherche(etatIndex, f, meta, n) {
  const morceaux = [etatIndex ?? "index : à jour"];
  if (f?.lag_files) morceaux.push(`${f.lag_files} fichier(s) en retard sur l'arbre`);
  if (f?.dirty) morceaux.push("arbre modifié depuis l'index");
  const comptes = [];
  if (meta.bm25Count != null) comptes.push(`mots ${meta.bm25Count}`);
  if (meta.vectorCount != null) comptes.push(`sens ${meta.vectorCount}`);
  if (meta.rerankedCount) comptes.push(`relus ${meta.rerankedCount}`);
  morceaux.push(`${n} résultat(s)${comptes.length ? ` (${comptes.join(", ")})` : ""}`);
  return morceaux.join(" · ");
}

// Le voisinage en papillon : ce qui en dépend à gauche (impact), ce qu'il
// appelle à droite (callees), une colonne par niveau ; une arête suit `from`,
// le nœud d'où le parcours l'a atteint. Un nœud sans `from` (atteint par le
// nom seul) se rattache au centre, en pointillé : « peut-être ».
const SVG = "http://www.w3.org/2000/svg";
function svg(tag, attrs = {}, ...enfants) {
  const e = document.createElementNS(SVG, tag);
  for (const [k, v] of Object.entries(attrs)) if (v !== undefined && v !== null) e.setAttribute(k, v);
  for (const c of enfants.flat()) e.append(c instanceof Node ? c : document.createTextNode(String(c)));
  return e;
}
const court = (t, n) => (t.length > n ? t.slice(0, n - 1) + "…" : t);

async function graphe(id, nom) {
  filDAriane(id, "", false);
  fil.append(" · ", el("a", { href: lienSymbole(id, nom) }, nom), " · graphe");
  const [imp, app] = await Promise.all([
    appeler(id, "impact", { name: nom, depth: 3, format: "json" }).catch(() => ({ starts: [], reached: [] })),
    appeler(id, "callees", { name: nom, depth: 3, format: "json" }).catch(() => ({ starts: [], reached: [] })),
  ]);
  const L = 210, H = 34, LARGE = 180, MARGE = 24;
  const centres = [...(imp.starts ?? []), ...(app.starts ?? [])].filter((d, i, t) => t.findIndex((x) => x.uuid === d.uuid) === i);
  const cotes = [{ r: imp, sens: -1 }, { r: app, sens: 1 }];
  const colonnes = new Map(); // "sens:niveau" → nœuds
  for (const { r, sens } of cotes) for (const x of r.reached ?? []) {
    const k = `${sens}:${x.level}`;
    if (!colonnes.has(k)) colonnes.set(k, []);
    colonnes.get(k).push(x);
  }
  const prof = Math.max(1, ...[...colonnes.keys()].map((k) => Number(k.split(":")[1])));
  const hauteur = Math.max(centres.length, ...[...colonnes.values()].map((c) => c.length), 1) * H + 2 * MARGE;
  const largeur = (2 * prof + 1) * L + 2 * MARGE;
  const pos = new Map(); // uuid du côté → {x, y}
  const place = (cle, items, colonne) => items.forEach((x, i) => {
    const y = MARGE + (hauteur - 2 * MARGE - items.length * H) / 2 + i * H;
    pos.set(cle(x), { x: MARGE + colonne * L, y, x0: x });
  });
  place((x) => `c:${x.uuid}`, centres, prof);
  for (const [k, items] of colonnes) {
    const [sens, niveau] = k.split(":").map(Number);
    place((x) => `${sens}:${x.uuid}`, items, prof + sens * niveau);
  }
  const traits = svg("g");
  const noeuds = svg("g");
  const ou = (sens, uuid) => pos.get(`${sens}:${uuid}`) ?? pos.get(`c:${uuid}`) ?? pos.get(`c:${centres[0]?.uuid}`);
  for (const { r, sens } of cotes) for (const x of r.reached ?? []) {
    const a = pos.get(`${sens}:${x.uuid}`);
    const b = x.from ? ou(sens, x.from) : ou(sens, centres[0]?.uuid);
    if (!a || !b) continue;
    // De gauche à droite : l'appelant vers l'appelé.
    const [g, d] = sens < 0 ? [a, b] : [b, a];
    const x1 = g.x + LARGE, y1 = g.y + H / 2 - 4, x2 = d.x, y2 = d.y + H / 2 - 4;
    traits.append(svg("path", { d: `M${x1},${y1} C${(x1 + x2) / 2},${y1} ${(x1 + x2) / 2},${y2} ${x2},${y2}`, class: x.from && !x.by_name ? "" : "devine" }));
  }
  for (const [cle, p] of pos) {
    const x = p.x0;
    const centre = cle.startsWith("c:");
    const lieu = `${x.path ?? ""}${x.line ? ":" + x.line : ""}`;
    noeuds.append(svg("g", { class: `noeud${centre ? " centre" : ""}`, transform: `translate(${p.x},${p.y})` },
      svg("title", {}, `${x.title} — ${lieu}${x.relation ? " — " + x.relation : ""}`),
      svg("rect", { width: LARGE, height: H - 8, rx: 5 }),
      svg("a", { href: centre ? lienSymbole(id, x.title) : lienGraphe(id, x.title) },
        svg("text", { x: 8, y: 12, class: "titre" }, court(x.title, 26))),
      svg("a", { href: lienFichier(id, x.path ?? "", x.line) },
        svg("text", { x: 8, y: 23, class: "lieu" }, court(lieu, 32)))));
  }
  const total = (imp.reached ?? []).length + (app.reached ?? []).length;
  vue.replaceChildren(
    el("h1", {}, nom),
    el("p", { class: "note" }, `${(imp.reached ?? []).length} en dépendent (à gauche), ${(app.reached ?? []).length} appelés (à droite), sur trois niveaux. Un nom mène à son propre graphe, un lieu à son fichier.`),
    total || centres.length
      ? el("div", { class: "graphe" }, svg("svg", { width: largeur, height: hauteur, viewBox: `0 0 ${largeur} ${hauteur}` }, traits, noeuds))
      : el("p", { class: "note" }, "Rien d'atteint depuis ce nom."),
  );
}

// ─── Les propositions (page 08) ─────────────────────────────────────────────
// Une proposition est un état de l'hôte : la page la montre et dit oui ou non.
// La liste se relit à chaque changement (sujet `declarations` du flux) ; le
// détail montre la différence textuelle de chaque fichier, le diff de chaque
// graphe touché sur son dessin (ajoutés, changés, retirés), l'aperçu d'un
// composant proposé sans l'appliquer, et Appliquer / Rejeter.

const etatDe = (p) => p.etat === "appliquee" ? `appliquée (${p.commit ? p.commit.slice(0, 9) : "rien à écrire"})`
  : p.etat === "rejetee" ? `rejetée : ${p.pourquoi}` : p.refus ? "refusée à la validation" : "proposée";

// Un graphe touché, avant et après, dessiné avec son diff : la réunion des
// deux, les nœuds ajoutés, changés et retirés surlignés (les propositions, la
// relecture d'une tâche).
function sectionDeGraphe(g) {
      const d = g.diff;
      const union = { nodes: [...(g.apres?.nodes ?? []), ...(g.avant?.nodes ?? []).filter((n) => d.retires.includes(n.name))],
        edges: [...(g.apres?.edges ?? []), ...(g.avant?.edges ?? []).filter((e) => d.aretes_retirees.includes(`${e.from_node}.${e.from_port}→${e.to_node}.${e.to_port}`))] };
      const cle = (e) => `${e.from_node}.${e.from_port}→${e.to_node}.${e.to_port}`;
      const { dessin } = dessinerGraphe(union,
        (n) => d.ajoutes.includes(n) ? "ajoute" : d.changes.includes(n) ? "change" : d.retires.includes(n) ? "retire" : "",
        (e) => d.aretes_ajoutees.includes(cle(e)) ? "ajoutee" : d.aretes_retirees.includes(cle(e)) ? "retiree" : "");
      const [genre, nom] = Object.entries(g.cible)[0];
      return el("section", { class: "graphe-propose", "data-cible": `${genre}:${nom}` },
        el("h2", {}, `graphe — ${genre} ${nom}`),
        el("p", { class: "note" }, `${d.ajoutes.length} ajouté(s), ${d.changes.length} changé(s), ${d.retires.length} retiré(s) ; ${d.aretes_ajoutees.length} arête(s) ajoutée(s), ${d.aretes_retirees.length} retirée(s).`),
        el("div", { class: "graphe" }, dessin));
}

// **Les bilans** (page 04 du 25 octobre 2026) : ce que le juge de session a
// rangé dans la mémoire, par fiche (une tâche) — ses apports dans l'ordre, la
// tendance de la note, et chaque bilan lisible tel qu'il est écrit.
async function bilans(id) {
  const b = backend(id);
  fil.replaceChildren(el("a", { href: "/ide/" }, "backends"), " · ", nomCourt(b.manifest), " · bilans");
  const r = await (await fetch(`/ide/bilans?${new URLSearchParams({ backend: id })}`)).json();
  if (!r.ok) throw new Error(r.error);
  const { declare, fiches } = r.result;
  vue.replaceChildren(
    el("h1", {}, "Bilans de session"),
    declare
      ? el("p", { class: "note" }, "Le juge relit chaque tour du chat de ce backend, et range ici son bilan, par tâche.")
      : el("p", { class: "note", id: "bilans-absents" }, "Ce backend ne déclare pas `bilans` : ses tours ne sont pas jugés."),
    ...(fiches.length || !declare ? [] : [el("p", { class: "note" }, "aucun bilan encore")]),
    ...fiches.map((f) => el("section", { class: "fiche-de-bilan", "data-fiche": f.fiche },
      el("h2", {}, f.fiche),
      el("p", { class: "note tendance" }, `notes : ${f.apports.map((a) => a.note ?? "?").join(" → ")}${f.apports.length > 1 ? ` · ${f.apports.at(-1).comparaison?.verdict ?? ""}` : ""}`),
      ...f.apports.map((a) => el("details", { class: "apport-de-bilan" },
        el("summary", {}, `note ${a.note ?? "?"} · ${a.source ?? ""}`),
        el("pre", {}, a.texte))))),
  );
}

async function propositions(id, reste) {
  if (reste) return proposition(id, Number(reste));
  const b = backend(id);
  fil.replaceChildren(el("a", { href: "/ide/" }, "backends"), " · ", nomCourt(b.manifest), " · propositions");
  const r = await (await fetch(`/ide/propositions?${new URLSearchParams({ backend: id })}`)).json();
  if (!r.ok) throw new Error(r.error);
  const liste = [...r.result].reverse();
  vue.replaceChildren(
    el("h1", {}, "Propositions"),
    el("p", { class: "note" }, "Ce que l'agent (ou quelqu'un) propose de déclarer : validé sans être écrit, appliqué ou rejeté ici."),
    liste.length
      ? el("table", { class: "runs", id: "propositions" },
          el("thead", {}, el("tr", {}, ...["", "raison", "auteur", "fichiers", "état"].map((t) => el("th", {}, t)))),
          el("tbody", {}, liste.map((p) => el("tr", { "data-id": p.id, "data-etat": p.refus && p.etat === "proposee" ? "refusee" : p.etat },
            el("td", {}, el("a", { href: `/ide/code/${id}/propositions/${p.id}` }, `#${p.id}`)),
            el("td", {}, p.raison),
            el("td", {}, p.auteur),
            el("td", {}, p.fichiers.map((f) => f.path + (f.neuf ? " (neuf)" : "")).join(", ")),
            el("td", {}, etatDe(p))))))
      : el("p", { class: "note" }, "Aucune proposition pour ce backend."),
  );
}

async function proposition(id, pid) {
  const b = backend(id);
  fil.replaceChildren(el("a", { href: "/ide/" }, "backends"), " · ",
    el("a", { href: `/ide/code/${id}/propositions/` }, nomCourt(b.manifest)), ` · proposition #${pid}`);
  const r = await (await fetch(`/ide/propositions?${new URLSearchParams({ backend: id, id: pid })}`)).json();
  if (!r.ok) throw new Error(r.error);
  const { proposition: p, graphes, composants } = r.result;
  const agir = async (route, corps) => {
    const rep = await (await fetch(`/ide/propositions/${route}`, {
      method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify({ backend: id, id: pid, ...corps }),
    })).json();
    if (!rep.ok) note.textContent = rep.error;
    else montrer();
  };
  const note = el("span", { class: "note", id: "proposition-note" }, "");
  const pourquoi = el("input", { id: "pourquoi", placeholder: "pourquoi rejeter", "aria-label": "pourquoi rejeter" });
  const ouverte = p.etat === "proposee";
  vue.replaceChildren(el("div", { class: "proposition", "data-etat": p.etat },
    el("h1", {}, `Proposition #${p.id}`),
    el("p", { class: "note" }, `${p.raison} — par ${p.auteur} — `, el("strong", { id: "proposition-etat" }, etatDe(p))),
    p.refus ? el("pre", { class: "refus" }, p.refus) : "",
    ouverte ? el("div", { class: "actions" },
      p.refus ? "" : el("button", { id: "appliquer", onclick: () => agir("appliquer", {}) }, "Appliquer"),
      pourquoi,
      el("button", { id: "rejeter", onclick: () => agir("rejeter", { why: pourquoi.value || "rejetée dans l'IDE" }) }, "Rejeter"),
      note) : "",
    composants.length ? el("p", {}, "Aperçu, sans appliquer : ", ...composants.map((c) =>
      el("a", { href: `/ide/apercu/${id}/${encodeURIComponent(c)}?proposition=${pid}`, target: "_blank", class: "apercu-propose" }, c))) : "",
    ...graphes.map(sectionDeGraphe),
    ...p.fichiers.map((f) => el("section", {},
      el("h2", {}, f.path + (f.avant == null ? " (neuf)" : "")),
      el("div", { class: "diff" }, difference(f.avant ?? "", f.apres)))),
  ));
}

// ─── Les tâches de l'agent (page 09) ────────────────────────────────────────
// Une tâche : une branche agent/<nom>, un worktree, une base à soi, et une
// seconde instance du backend (ses vues sont celles de tout backend). La
// relecture montre ce que sa branche change, ses graphes en diff, ses témoins
// rejoués, puis Fusionner (en avance rapide) ou Rejeter.

async function taches(id, reste) {
  if (reste) return tache(id, reste);
  const b = backend(id);
  fil.replaceChildren(el("a", { href: "/ide/" }, "backends"), " · ", nomCourt(b.manifest), " · tâches");
  const r = await (await fetch(`/ide/taches?${new URLSearchParams({ backend: id })}`)).json();
  if (!r.ok) throw new Error(r.error);
  const { ouvertes, fermees } = r.result;
  const note = el("span", { class: "note", id: "tache-note" }, "");
  const nom = el("input", { id: "tache-nom", placeholder: "nom de la tâche", "aria-label": "nom de la tâche" });
  const ouvrir = async () => {
    note.textContent = "copie de la base, worktree…";
    const rep = await (await fetch("/ide/taches", { method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify({ backend: id, nom: nom.value }) })).json();
    if (!rep.ok) { note.textContent = rep.error; return; }
    await chargerEtat();
    montrer();
  };
  vue.replaceChildren(
    el("h1", {}, "Tâches de l'agent"),
    el("p", { class: "note" }, "Une tâche : une branche à elle, un worktree, une copie de la base. L'agent y déclare ; on relit, puis on fusionne ou on rejette."),
    el("div", { class: "barre" }, nom, el("button", { id: "ouvrir-tache", onclick: ouvrir }, "Ouvrir une tâche"), note),
    ouvertes.length
      ? el("table", { class: "runs", id: "taches" },
          el("thead", {}, el("tr", {}, ...["tâche", "branche", "base", "backend"].map((t) => el("th", {}, t)))),
          el("tbody", {}, ouvertes.map((t) => el("tr", { "data-nom": t.nom },
            el("td", {}, el("a", { href: `/ide/code/${id}/taches/${encodeURIComponent(t.nom)}` }, t.nom)),
            el("td", {}, t.branche),
            el("td", {}, `${t.copie.reflink ? "reflink" : "copie pleine"}, ${Math.round(t.copie.octets / 1024)} Kio, porte ${t.copie.tenue_ms} ms`),
            el("td", {}, el("a", { href: `/ide/code/${t.id}/arbre/` }, "ses vues"), " · ", el("a", { href: `/ide/code/${t.id}/propositions/` }, "ses propositions"))))))
      : el("p", { class: "note" }, "Aucune tâche ouverte."),
    fermees.length ? el("h2", {}, "Fermées") : "",
    fermees.length ? el("ul", { class: "liste", id: "taches-fermees" }, fermees.map((t) => el("li", { "data-nom": t.nom, "data-etat": t.etat },
      t.nom, el("span", { class: "meta" }, t.etat === "fusionnee" ? `fusionnée (${String(t.commit).slice(0, 9)})` : t.etat === "orpheline" ? `orpheline : ${t.pourquoi}` : `rejetée : ${t.pourquoi}`)))) : "",
  );
}

async function tache(id, nom) {
  const b = backend(id);
  fil.replaceChildren(el("a", { href: "/ide/" }, "backends"), " · ",
    el("a", { href: `/ide/code/${id}/taches/` }, nomCourt(b.manifest)), ` · tâche ${nom}`);
  const r = await (await fetch(`/ide/taches?${new URLSearchParams({ backend: id, nom })}`)).json();
  if (!r.ok) throw new Error(r.error);
  const { tache: t, fichiers, graphes, temoins, base_a_bouge } = r.result;
  const note = el("span", { class: "note", id: "tache-note" }, "");
  const pourquoi = el("input", { id: "pourquoi", placeholder: "pourquoi rejeter", "aria-label": "pourquoi rejeter" });
  const message = el("input", { id: "message", placeholder: `message (la tâche ${nom})`, "aria-label": "message de la fusion" });
  const agir = async (route, corps) => {
    note.textContent = route === "fusionner" ? "fusion…" : "rejet…";
    const rep = await (await fetch(`/ide/taches/${route}`, { method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify({ backend: id, nom, ...corps }) })).json();
    if (!rep.ok) { note.textContent = rep.error; return; }
    await chargerEtat();
    history.pushState(null, "", `/ide/code/${id}/taches/`);
    montrer();
  };
  vue.replaceChildren(el("div", { class: "proposition" },
    el("h1", {}, `Tâche ${t.nom}`),
    el("p", { class: "note" }, `${t.branche}, partie de ${t.depart.slice(0, 9)}${base_a_bouge ? " — la base du dépôt a bougé depuis : la fusion rebasera" : ""}. `,
      el("a", { href: `/ide/code/${t.id}/arbre/` }, "ses vues")),
    el("p", { id: "temoins" }, temoins.chargement ? "Témoins : la branche se charge entière." : "Témoins : la branche ne se charge pas."),
    temoins.refus ? el("pre", { class: "refus" }, temoins.refus) : "",
    el("div", { class: "actions" },
      message,
      el("button", { id: "fusionner", onclick: () => agir("fusionner", { message: message.value }) }, "Fusionner"),
      pourquoi,
      el("button", { id: "rejeter", onclick: () => agir("rejeter", { why: pourquoi.value || "rejetée dans l'IDE" }) }, "Rejeter"),
      note),
    fichiers.length ? "" : el("p", { class: "note" }, "La branche ne change encore rien."),
    ...graphes.map(sectionDeGraphe),
    ...fichiers.map((f) => el("section", { class: "fichier-de-tache", "data-genre": f.genre, "data-path": f.path },
      el("h2", {}, (f.genre === "code" ? "code · " : f.genre === "depot" ? "dépôt · " : "") + f.path + (f.avant == null ? " (neuf)" : f.apres == null ? " (retiré)" : "")),
      el("div", { class: "diff" }, difference(f.avant ?? "", f.apres ?? "")))),
  ));
}

// ─── Les réactions ((b)) ───────────────────────────────────────────────────
// Les réactions du backend (/ide/reactions), chacune menant à son DAG vivant ;
// l'état des montres et le dernier fichier qu'elles ont vu ; les runs récents,
// tenus par le flux : un `RunStarted` dont la cible est une réaction ouvre une
// ligne, son `RunFinished` la ferme.

let reactionsVivantes = null; // { manifest, corps: <tbody>, lignes: Map(run → <tr>), vu: <p> }

async function reactions(id) {
  const b = backend(id);
  fil.replaceChildren(el("a", { href: "/ide/" }, "backends"), " · ", nomCourt(b.manifest), " · réactions");
  const r = await (await fetch(`/ide/reactions?${new URLSearchParams({ backend: id })}`)).json();
  if (!r.ok) throw new Error(r.error);
  const d = r.result;
  const lag = d.index?.lag?.files ?? 0;
  const montres = Object.entries(d.index?.watches ?? {});
  const corps = el("tbody");
  const vu = el("p", { class: "note", id: "montre-vue" }, "Aucun fichier vu depuis l'ouverture de cette page.");
  vue.replaceChildren(
    el("h1", {}, "Réactions"),
    el("p", { class: "note" }, `${d.runs} run(s) depuis l'ouverture du backend${d.errors.length ? `, ${d.errors.length} erreur(s)` : ""} ; retard des fichiers : ${lag}.`),
    d.reactions.length
      ? el("ul", { class: "liste" }, d.reactions.map((x) => el("li", {},
          el("a", { href: `/ide/code/${id}/dag/reaction/${encodeURIComponent(x.name)}` }, x.name),
          el("span", { class: "meta" }, `sur ${x.on.join(", ")} · ${x.policy}`),
          el("span", { class: "meta" }, x.description))))
      : el("p", { class: "note" }, "Ce backend ne déclare aucune réaction."),
    el("h2", {}, "Montres"),
    montres.length
      ? el("ul", { class: "liste" }, montres.map(([n, w]) => el("li", {},
          n,
          el("span", { class: "meta" }, w.paused ? `en pause : ${w.paused}` : `${w.scans ?? 0} balayages, ${w.lists ?? 0} listes`))))
      : el("p", { class: "note" }, "Aucune montre en service."),
    vu,
    el("h2", {}, "Runs récents"),
    el("table", { class: "runs", id: "runs-recents" },
      el("thead", {}, el("tr", {}, el("th", {}, "début"), el("th", {}, "réaction"), el("th", {}, "run"), el("th", {}, "fin"), el("th", {}, "écrit"))),
      corps),
  );
  reactionsVivantes = { manifest: b.manifest, corps, lignes: new Map(), ecrits: new Map(), vu };
}

// Ce qu'une écriture du catalogue dit, cumulé par run : l'entité et le
// changement (les états d'une transition), ou les liens posés et retirés d'une
// relation. Une écriture porte le run du nœud qui l'a faite (la pièce 1 de la
// mémoire) : c'est ce qui la range sous le run de la réaction.
function cleDEcriture(ev) {
  if (ev.kind === "RelationsChanged") return `lien|${ev.relation}`;
  const etats = ev.change === "transitioned" && ev.from_state != null ? ` (${ev.from_state} → ${ev.to_state})` : "";
  return `entite|${ev.entity}|${ev.change}${etats}`;
}
function cumulerUneEcriture(acc, ev) {
  if (ev.kind === "RelationsChanged") {
    acc.plus = (acc.plus ?? 0) + (ev.added?.length ?? 0);
    acc.moins = (acc.moins ?? 0) + (ev.removed?.length ?? 0);
    acc.tronque = acc.tronque || ev.truncated;
    acc.attrs = { "data-relation": ev.relation };
    acc.texte = `${ev.relation} : +${acc.plus} −${acc.moins}${acc.tronque ? " (tronqué : le compte entier est dans le catalogue)" : ""}`;
  } else {
    acc.n = (acc.n ?? 0) + (ev.count ?? ev.uuids?.length ?? 1);
    const etats = ev.change === "transitioned" && ev.from_state != null ? ` (${ev.from_state} → ${ev.to_state})` : "";
    acc.attrs = { "data-entity": ev.entity, "data-change": ev.change };
    acc.texte = `${ev.entity} : ${acc.n} ${ev.change}${etats}`;
  }
  return acc;
}

// Un événement du flux, pour la vue des réactions.
function suivreLesReactions(d) {
  const v = reactionsVivantes;
  const ev = d.evenement ?? {};
  if (!v || route().vue !== "reactions" || d.backend !== v.manifest) return;
  const heure = new Date(d.at_ms ?? Date.now()).toLocaleTimeString();
  if (ev.kind === "FileChanged") {
    v.vu.textContent = `Dernier fichier vu par la montre « ${ev.watch} » : ${ev.path} (${ev.change}, ${heure}).`;
  } else if (ev.kind === "RunStarted" && ev.target?.reaction) {
    const fin = el("td", { class: "fin" }, "en cours ",
      el("button", { class: "arreter", onclick: (e) => arreterUnRun(route().id, ev.run, e.target) }, "Arrêter"));
    const ligne = el("tr", { "data-reaction": ev.target.reaction, "data-run": ev.run },
      el("td", {}, heure),
      el("td", {}, el("a", { href: `/ide/code/${route().id}/dag/reaction/${encodeURIComponent(ev.target.reaction)}` }, ev.target.reaction)),
      el("td", {}, ev.run),
      fin,
      el("td", { class: "ecrit" }, el("ul", { class: "ecritures" })));
    v.corps.prepend(ligne);
    v.lignes.set(ev.run, fin);
    v.ecrits.set(ev.run, { liste: ligne.querySelector("ul.ecritures"), parCle: new Map() });
    while (v.corps.children.length > 50) {
      const run = v.corps.lastElementChild.dataset.run;
      v.lignes.delete(run), v.ecrits.delete(run), v.corps.lastElementChild.remove();
    }
  } else if ((ev.kind === "EntitiesChanged" || ev.kind === "RelationsChanged") && v.ecrits.has(ev.run)) {
    // Les écritures d'une même entité et d'un même changement se cumulent sur
    // une ligne, celles d'une même relation aussi.
    const e = v.ecrits.get(ev.run);
    const cle = cleDEcriture(ev);
    const acc = cumulerUneEcriture(e.parCle.get(cle) ?? {}, ev);
    if (!acc.li) {
      acc.li = el("li", acc.attrs);
      e.liste.append(acc.li);
      e.parCle.set(cle, acc);
    }
    acc.li.textContent = acc.texte;
  } else if (ev.kind === "RunFinished" && v.lignes.has(ev.run)) {
    const fin = v.lignes.get(ev.run);
    fin.textContent = finDuRun(ev);
    if (!ev.ok) fin.classList.add("echec");
  }
}

// ─── Le chat de l'atelier (page 11) ────────────────────────────────────────
// L'agent dont l'objet est le gabarit, servi par l'hôte. La page n'a pas
// d'état que l'hôte n'ait pas : le fil vient de GET /ide/chat ; pendant un
// tour, le flux montre ses jetons, ses outils et ses propositions, et Arrêter
// passe par le même chemin que tout run (C4).

let chatVivant = null; // { id, manifest, fil, run, bulle, outils, arreter, note, conso }
// Ce que le flux a dit du dernier tour (appels de modèle, appels d'outil,
// avec leurs durées) : lisible par un témoin, comme `window.__apercu`.
window.__chat = { run: null, evenements: [] };

function tourDuChat(t) {
  const appels = (t.tool_calls ?? []).map((c) => el("li", { "data-outil": c.name }, c.name));
  return el("div", { class: `tour ${t.role}`, "data-role": t.role },
    t.content ? el("pre", {}, t.content) : "",
    appels.length ? el("ul", { class: "outils" }, appels) : "");
}

async function chat(id) {
  const b = backend(id);
  fil.replaceChildren(el("a", { href: "/ide/" }, "backends"), " · ", nomCourt(b.manifest), " · chat");
  const r = await (await fetch(`/ide/chat?${new URLSearchParams({ backend: id })}`)).json();
  if (!r.ok) throw new Error(r.error);
  const s = r.result;
  const filDuChat = el("div", { class: "chat", id: "chat-fil" }, s.turns.filter((t) => t.role !== "tool").map(tourDuChat));
  const note = el("span", { class: "note", id: "chat-note" }, s.on ? `modèle : ${s.model}` : `chat éteint : ${s.why}`);
  const message = el("textarea", { id: "chat-message", rows: 3, placeholder: "Demandez à l'agent…", "aria-label": "message à l'agent" });
  message.disabled = !s.on;
  const envoyer = el("button", { id: "chat-envoyer", onclick: () => envoyerAuChat(id) }, "Envoyer");
  envoyer.disabled = !s.on;
  const arreter = el("button", { id: "chat-arreter", onclick: () => chatVivant?.run && arreterUnRun(id, chatVivant.run, note) }, "Arrêter");
  arreter.hidden = !s.run;
  const conso = el("p", { class: "note", id: "chat-conso" }, "");
  vue.replaceChildren(
    el("h1", {}, "Chat de l'atelier"),
    el("p", { class: "note" }, b.tache
      ? "Une tâche : l'agent déclare sur sa branche ; la relecture et la fusion sont dans les tâches."
      : "L'agent propose ; vous appliquez ou rejetez dans les propositions."),
    s.orphans?.length ? el("p", { class: "alerte", id: "chat-orphelines" },
      `${s.orphans.length} session(s) orpheline(s), jamais rouvertes : ${s.orphans.map((o) => o.pourquoi).join(" ; ")}`) : "",
    filDuChat,
    el("div", { class: "barre" }, message, envoyer, arreter, note),
    conso,
  );
  chatVivant = { id, manifest: b.manifest, fil: filDuChat, run: s.run ?? null, bulle: null, outils: null, arreter, note, conso };
}

async function envoyerAuChat(id) {
  const v = chatVivant;
  const m = document.getElementById("chat-message");
  const texte = m.value.trim();
  if (!v || !texte) return;
  m.value = "";
  v.fil.append(tourDuChat({ role: "user", content: texte }));
  v.outils = el("ul", { class: "outils" });
  v.bulle = el("div", { class: "tour assistant en-cours", "data-role": "assistant" }, el("pre", {}, ""), v.outils);
  v.fil.append(v.bulle);
  v.note.textContent = "l'agent travaille…";
  const r = await (await fetch("/ide/chat", {
    method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify({ backend: id, message: texte }),
  })).json();
  v.bulle.classList.remove("en-cours");
  v.arreter.hidden = true;
  v.run = null;
  if (!r.ok) { v.note.textContent = r.error; return; }
  const res = r.result;
  if (res.text) v.bulle.querySelector("pre").textContent = res.text;
  // Les propositions du tour viennent aussi de sa réponse : le flux les dit
  // sur `declarations`, relayé à part d'`agent`, et peut les donner après la
  // fin du tour.
  for (const p of res.propositions ?? []) lienDeProposition(v, v.bulle, p);
  v.bulle.dataset.run = res.run;
  // Un tour d'une tâche qui a écrit : ses fichiers, sur la branche, et la
  // relecture de la tâche (diff, témoins, fusion).
  if (res.tache && res.fichiers?.length) {
    v.bulle.append(el("p", { class: "tache-du-tour" },
      `${res.fichiers.join(", ")} — sur la branche ${res.tache.branche} · `,
      el("a", { href: `/ide/code/${res.tache.parent}/taches/${encodeURIComponent(res.tache.nom)}`, "data-tache": res.tache.nom }, "relire la tâche")));
  }
  v.note.textContent = res.stopped_by ? `arrêté ${res.stopped_by === "ide" ? "dans l'IDE" : `par ${res.stopped_by}`}` : "fini";
  v.conso.textContent = res.consumed?.length
    ? `consommé : ${res.consumed.map((c) => `${c.slug} ${c.amount} ${c.unit}`).join(", ")}`
    : "";
}

// Le lien d'une proposition du tour dans sa bulle (`p` : id, paths,
// accepted), une fois : le flux et la réponse du tour la disent tous deux.
function lienDeProposition(v, bulle, p) {
  if (bulle.querySelector(`a[data-proposition="${CSS.escape(String(p.id))}"]`)) return;
  bulle.append(el("p", { class: "proposition-du-tour" },
    el("a", { href: `/ide/code/${v.id}/propositions/${p.id}`, "data-proposition": p.id }, `proposition #${p.id}`),
    ` — ${(p.paths ?? []).join(", ")}${p.accepted === false ? " (refusée à la validation)" : ""}`));
}

// Un événement du flux, pour le chat montré.
function suivreLeChat(d) {
  const v = chatVivant;
  const ev = d.evenement ?? {};
  if (!v || route().vue !== "chat" || d.backend !== v.manifest) return;
  if (ev.kind === "RunStarted" && ev.run_kind === "agent" && ev.name === `chat:${v.id}`) {
    v.run = ev.run;
    if (v.bulle) v.bulle.dataset.run = ev.run;
    window.__chat = { run: ev.run, evenements: [] };
    v.arreter.hidden = false;
    return;
  }
  // Une proposition faite pendant le tour : un lien vers elle, dans la bulle.
  if (ev.kind === "PropositionChanged" && ev.change === "proposed" && v.bulle && v.run) {
    lienDeProposition(v, v.bulle, ev);
    return;
  }
  // La bulle de ce run : celle du tour en cours, ou une bulle déjà finie —
  // le flux relaie chaque sujet à part, et peut dire un appel d'outil après
  // la réponse du tour.
  if (!ev.run) return;
  const bulle = v.fil.querySelector(`.tour.assistant[data-run="${CSS.escape(String(ev.run))}"]`);
  if (!bulle) return;
  if (d.sujet === "agent" && window.__chat.run === ev.run) {
    window.__chat.evenements.push({ kind: ev.kind, at_ms: d.at_ms, ms: ev.ms ?? null, tool: ev.tool ?? null,
      iteration: ev.iteration ?? null, prompt_tokens: ev.prompt_tokens ?? null, completion_tokens: ev.completion_tokens ?? null,
      retries: ev.retries ?? null, finish: ev.finish ?? null, ok: ev.ok ?? null });
  }
  const outils = bulle.querySelector(".outils");
  if (ev.kind === "Token") {
    // Pendant le tour seulement : sa réponse pose ensuite le texte entier.
    if (v.run === ev.run) bulle.querySelector("pre").textContent += ev.text;
  } else if (ev.kind === "ToolCallStarted" && outils) {
    outils.append(el("li", { "data-outil": ev.tool, "data-appel": ev.call_id }, `${ev.tool}…`));
  } else if (ev.kind === "ToolCallFinished" && outils) {
    const li = outils.querySelector(`li[data-appel="${CSS.escape(String(ev.call_id))}"]`);
    if (li) li.textContent = `${ev.tool} : ${ev.ok === false ? `refusé (${ev.error_kind ?? "erreur"})` : "fait"}`;
  } else if (ev.kind === "RunFinished" && v.run === ev.run) {
    v.arreter.hidden = true;
  }
}

// À chaque ouverture du flux, le chat montré relit son état : un tour parti
// avant (la page à peine chargée, ou le flux coupé puis repris) a publié son
// RunStarted à un flux qui n'avait pas cette fenêtre. Le run en vol que l'hôte
// connaît devient celui de la page, Arrêter avec. Seulement pendant un tour
// que la page attend : une réponse tardive ne rallume pas un tour fini.
async function relireLeChat() {
  const v = chatVivant;
  if (!v || route().vue !== "chat") return;
  const r = await (await fetch(`/ide/chat?${new URLSearchParams({ backend: v.id })}`)).json();
  const run = r.ok ? r.result.run : null;
  if (!run || v !== chatVivant || !v.bulle?.classList.contains("en-cours") || v.run === run) return;
  v.run = run;
  v.bulle.dataset.run = run;
  window.__chat = { run, evenements: [] };
  v.arreter.hidden = false;
}
// Pour un témoin : le flux de la page, et sa relecture du chat.
window.__ide = { relireLeChat };

// ─── Le DAG vivant (C2) ────────────────────────────────────────────────────
// Le graphe d'un outil, d'une réaction ou d'un composant (/ide/graphe), dessiné
// par couches de gauche à droite ; le sujet `dataflow` du flux l'allume. Le run
// se reconnaît à sa cible (`RunStarted.target`, la même forme que la demande du
// graphe), un nœud au nom que porte son `NodeRun` — sur `dataflow` pour un
// appel d'outil, `reactions` pour une réaction.

let dagVivant = null; // { manifest, cible, run, noeuds: Map(nom → <g>), etat: <p> }

async function graphes(id) {
  const b = backend(id);
  fil.replaceChildren(el("a", { href: "/ide/" }, "backends"), " · ", nomCourt(b.manifest), " · graphes");
  const r = await (await fetch("/ide/describe", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ manifest: b.manifest }),
  })).json();
  const d = r.result ?? r;
  if (r.ok === false) throw new Error(r.error ?? "describe refusé");
  const outils = (d.tools ?? []).map((t) => t.name).sort();
  vue.replaceChildren(
    el("h1", {}, "Graphes des outils"),
    el("p", { class: "note" }, "Un outil appelé pendant qu'on regarde son graphe s'y allume, nœud par nœud."),
    el("ul", { class: "liste" }, outils.map((n) => el("li", {}, el("a", { href: `/ide/code/${id}/dag/tool/${encodeURIComponent(n)}` }, n)))),
  );
}

// Le dessin d'un graphe (la forme de /ide/graphe), par couches de gauche à
// droite : la couche d'un nœud est la plus longue chaîne d'arêtes qui y mène
// (bornée par le nombre de nœuds : un cycle ne fait pas tourner la page).
// `classeDe(nom)` et `classeArete(e)` ajoutent leurs classes (le diff d'une
// proposition). Rend le <svg> et les <g> des nœuds par nom.
function dessinerGraphe(g, classeDe = () => "", classeArete = () => "") {
  const couche = new Map(g.nodes.map((n) => [n.name, 0]));
  for (let i = 0; i < g.nodes.length; i++) for (const e of g.edges) {
    const c = (couche.get(e.from_node) ?? 0) + 1;
    if (couche.has(e.to_node) && c > couche.get(e.to_node) && c < g.nodes.length) couche.set(e.to_node, c);
  }
  const colonnes = [];
  for (const n of g.nodes) (colonnes[couche.get(n.name)] ??= []).push(n);
  const L = 250, H = 56, LARGE = 200, MARGE = 24;
  const hauteur = Math.max(1, ...Array.from(colonnes, (c) => c?.length ?? 0)) * H + 2 * MARGE;
  const largeur = Math.max(1, colonnes.length) * L + 2 * MARGE;
  const pos = new Map();
  colonnes.forEach((c, x) => (c ?? []).forEach((n, i) => pos.set(n.name, { x: MARGE + x * L, y: MARGE + i * H })));
  const traits = svg("g");
  for (const e of g.edges) {
    const a = pos.get(e.from_node), z = pos.get(e.to_node);
    if (!a || !z) continue;
    const x1 = a.x + LARGE, y1 = a.y + (H - 12) / 2, x2 = z.x, y2 = z.y + (H - 12) / 2;
    traits.append(
      svg("path", { d: `M${x1},${y1} C${(x1 + x2) / 2},${y1} ${(x1 + x2) / 2},${y2} ${x2},${y2}`, class: classeArete(e) }),
      svg("text", { x: (x1 + x2) / 2, y: (y1 + y2) / 2 - 3, "text-anchor": "middle", class: "port" },
        e.from_port === e.to_port ? e.from_port : `${e.from_port}→${e.to_port}`));
  }
  const noeuds = svg("g");
  const parNom = new Map();
  for (const n of g.nodes) {
    const p = pos.get(n.name);
    const etape = svg("g", { class: `etape ${classeDe(n.name)}`.trim(), "data-nom": n.name, transform: `translate(${p.x},${p.y})` },
      svg("title", {}, `${n.node_type}${n.line ? " — ligne " + n.line : ""}\n${JSON.stringify(n.config)}`),
      svg("rect", { width: LARGE, height: H - 12, rx: 5 }),
      svg("text", { x: 8, y: 14, class: "titre" }, court(n.name, 28)),
      svg("text", { x: 8, y: 27, class: "lieu" }, court(n.node_type, 30)),
      svg("text", { x: LARGE - 8, y: 27, "text-anchor": "end", class: "duree" }, ""));
    parNom.set(n.name, etape);
    noeuds.append(etape);
  }
  return { dessin: svg("svg", { width: largeur, height: hauteur, viewBox: `0 0 ${largeur} ${hauteur}` }, traits, noeuds), parNom };
}

async function dag(id, reste) {
  const [genre, ...suite] = reste.split("/");
  const nom = suite.join("/");
  if (!genre || !nom) return graphes(id);
  const b = backend(id);
  fil.replaceChildren(el("a", { href: "/ide/" }, "backends"), " · ",
    el("a", { href: `/ide/code/${id}/dag/` }, nomCourt(b.manifest)), ` · ${genre} ${nom}`);
  const r = await (await fetch(`/ide/graphe?${new URLSearchParams({ backend: id, [genre]: nom })}`)).json();
  if (!r.ok) throw new Error(r.error);
  const g = r.result;
  const { dessin, parNom } = dessinerGraphe(g);
  const etatDuRun = el("p", { class: "note", id: "dag-etat" }, "En attente d'un run de ce graphe…");
  const arretNote = el("span", { class: "note", id: "arret-note" }, "");
  const arreter = el("button", { id: "arreter-run", hidden: true, onclick: () => dagVivant?.run && arreterUnRun(id, dagVivant.run, arretNote) }, "Arrêter");
  vue.replaceChildren(
    el("h1", {}, `${nom}`),
    el("p", { class: "note" }, `${genre === "tool" ? "Outil" : genre === "reaction" ? "Réaction" : "Composant"}, ${g.nodes.length} nœud(s), ${g.edges.length} arête(s)${g.source ? " — " + g.source : ""}.`),
    etatDuRun,
    el("p", { class: "barre" }, arreter, arretNote),
    g.nodes.length
      ? el("div", { class: "graphe" }, dessin)
      : el("p", { class: "note" }, "Ce graphe est vide."),
  );
  dagVivant = { manifest: b.manifest, cible: JSON.stringify({ [genre]: nom }), run: null, noeuds: parNom, etat: etatDuRun, arreter, arretNote };
}

// **Arrêter un run depuis la page** (C4) : le même chemin d'arrêt que tout run,
// au nom de l'IDE. La fin du run (`RunFinished`, `cancelled_by`) le dira ; un
// refus (le run a fini entre-temps) se dit dans `note`.
async function arreterUnRun(id, run, note) {
  note.textContent = "arrêt demandé…";
  const r = await (await fetch("/ide/runs/arreter", {
    method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify({ backend: id, run }),
  })).json();
  // N'efface que son propre avis : la fin du run a pu écrire la sienne avant.
  if (!r.ok) note.textContent = r.error;
  else if (note.textContent === "arrêt demandé…") note.textContent = "";
}

// La fin d'un run, dite : arrêté (par qui), fini ou échoué.
function finDuRun(ev) {
  if (ev.cancelled_by) return `arrêté ${ev.cancelled_by === "ide" ? "dans l'IDE" : `par ${ev.cancelled_by}`} après ${ev.ms} ms`;
  return `${ev.ok ? "fini" : "échoué"} en ${ev.ms} ms`;
}

// Un événement `dataflow` du flux, pour le DAG montré.
function vivre(d) {
  const v = dagVivant;
  const ev = d.evenement ?? {};
  if (!v || route().vue !== "dag" || d.backend !== v.manifest) return;
  if (ev.kind === "RunStarted" && ev.target && JSON.stringify(ev.target) === v.cible) {
    v.run = ev.run;
    for (const g of v.noeuds.values()) {
      g.classList.remove("passe", "echec");
      g.querySelector("text.duree").textContent = "";
    }
    v.etat.textContent = `run ${ev.run} : en cours`;
    v.arreter.hidden = false;
    v.arretNote.textContent = "";
    return;
  }
  if (!v.run || ev.run !== v.run) return;
  if (ev.kind === "NodeRun") {
    const g = v.noeuds.get(ev.node);
    if (!g) return;
    g.classList.add(ev.ok === false || ev.error ? "echec" : "passe");
    g.querySelector("text.duree").textContent = `${ev.ms} ms`;
  } else if (ev.kind === "RunFinished") {
    v.etat.textContent = `run ${ev.run} : ${finDuRun(ev)}`;
    v.arreter.hidden = true;
  }
}

// ─── Le routage, et le flux ───────────────────────────────────────────────

function route() {
  const p = location.pathname.replace(/\/+$/, "") || "/ide";
  const m = p.match(/^\/ide\/code\/([^/]+)\/(arbre|fichier|symbole|graphe|recherche|dag|reactions|propositions|taches|chat|bilans)(?:\/(.*))?$/);
  if (!m) return { vue: "accueil" };
  return { vue: m[2], id: m[1], reste: decodeURIComponent(m[3] ?? "") };
}

async function montrer() {
  const r = route();
  lignesDEtat = new Set();
  try {
    await vueDe(r);
  } catch (e) { erreur(e); }
  if (lignesDEtat.size) vue.prepend(...[...lignesDEtat].map((l) => el("p", { class: "note etat" }, l)));
  const derive = r.id ? noteDeDerive(backend(r.id)) : null;
  if (derive) vue.prepend(derive);
}

// Les copies locales de gabarits livrés qui en diffèrent (`template_drift` de
// l'état) : dites en tête des vues du backend, avant qu'un outil d'avant ne
// refuse un argument (l'IDE de Lucie, 19 octobre 2026).
function noteDeDerive(b) {
  const d = b?.template_drift ?? [];
  if (!d.length) return null;
  return el("p", { class: "note derive" },
    `${d.length} fichier${d.length > 1 ? "s" : ""} de ce backend diffère${d.length > 1 ? "nt" : ""} du gabarit livré du même nom : `,
    d.slice(0, 6).map((x) => x.builtin).join(", "), d.length > 6 ? "…" : "",
    ". Si ce n'est pas voulu : rag3weaver-backend templates link <manifeste>.");
}

async function vueDe(r) {
  if (!etat) await chargerEtat();
  if (r.id && !backend(r.id)) throw new Error(`L'hôte ne connaît pas le backend ${r.id} (ouvert depuis ?). Revenir à l'accueil.`);
  switch (r.vue) {
    case "arbre": return await arbre(r.id, r.reste);
    case "fichier": return await fichier(r.id, r.reste);
    case "symbole": return await symbole(r.id, r.reste);
    case "graphe": return await graphe(r.id, r.reste);
    case "recherche": return await recherche(r.id);
    case "dag": return await dag(r.id, r.reste);
    case "reactions": return await reactions(r.id);
    case "propositions": return await propositions(r.id, r.reste);
    case "taches": return await taches(r.id, r.reste);
    case "chat": return await chat(r.id);
    case "bilans": return await bilans(r.id);
    default: return await accueil();
  }
}
montrer();

// Le flux : un fichier qui change se relit dans la vue qui le montre ; un run
// allume le DAG montré.
const lien = document.getElementById("lien");
const source = new EventSource("/ide/flux?sujets=files,dataflow,catalog,reactions,declarations,agent,tokens");
source.onopen = () => { lien.textContent = "flux : ouvert"; relireLeChat().catch(() => {}); };
window.__ide.flux = source;
source.onerror = () => { lien.textContent = "flux : coupé, reprise…"; };
let relire = null;
source.onmessage = (e) => {
  const d = JSON.parse(e.data);
  // Le chat : son run d'agent, ses appels d'outil et sa consommation sur
  // `agent`, ses jetons sur `tokens`, ses propositions sur `declarations`.
  suivreLeChat(d);
  if (d.sujet === "agent" || d.sujet === "tokens") return;
  // Les runs passent par `dataflow` (un appel d'outil par l'hôte), `reactions`
  // (une réaction) ou `catalog` (une ingestion) : le DAG vivant les lit tous.
  // Une proposition a changé : la vue des propositions se relit.
  if (d.sujet === "declarations") {
    if (route().vue === "propositions" && d.backend === backend(route().id)?.manifest) montrer();
    return;
  }
  suivreLesReactions(d);
  if (d.sujet === "dataflow" || d.sujet === "reactions" || d.sujet === "catalog") return vivre(d);
  const ev = d.evenement ?? {};
  const r = route();
  const b = r.id && backend(r.id);
  if (!b || d.backend !== b.manifest || ev.kind !== "FileChanged") return;
  const touche = r.vue === "fichier" ? ev.path === r.reste : r.vue === "arbre" ? ev.path?.startsWith(r.reste) : false;
  if (!touche) return;
  // En édition : un tampon modifié n'est jamais remplacé ; on le dit (page 05 §4).
  if (edition && r.vue === "fichier" && edition.chemin === ev.path) {
    // Notre propre enregistrement : le disque est déjà le tampon.
    if (ev.hash && ev.hash === edition.hash) return;
    if (edition.modifie) edition.avis.textContent = "changé sur le disque depuis la lecture : l'enregistrement montrera la différence";
    else editer(edition.id, edition.chemin);
    return;
  }
  clearTimeout(relire);
  relire = setTimeout(() => { const y = scrollY; montrer().then(() => scrollTo(0, y)); }, 300);
};
source.addEventListener("lacune", () => montrer());
// Une page quittée ferme son flux : gardée en cache d'historique, elle le tenait
// ouvert, et le navigateur ne donne que six connexions à un hôte — la septième vue
// attendait sans fin (vu par le témoin navigateur, 19 octobre 2026). Revenue de
// ce cache, elle se recharge plutôt que de montrer un état sans flux.
addEventListener("pagehide", () => source.close());
addEventListener("pageshow", (e) => { if (e.persisted) location.reload(); });
