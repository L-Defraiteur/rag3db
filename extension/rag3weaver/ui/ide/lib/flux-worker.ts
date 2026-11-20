/// **Le flux partagé de l'IDE** (Lucie, 13 novembre 2026 : plusieurs onglets de l'IDE ouverts
/// en même temps ; mesure de l'étape navigateur, `docs/everything-declarative/13-11-2026/
/// 02-mesure-de-l-etape-navigateur.md`). Un `SharedWorker` que toutes les pages d'un même site
/// partagent tient **un seul** flux de l'hôte (`/ide/flux`) : le navigateur n'ouvre que six
/// connexions HTTP/1.1 vers un hôte, et six onglets qui tenaient chacun le leur les prenaient
/// toutes.
///
/// - Chaque page s'y branche par un port et s'abonne à ses sujets (`abonner`, aucun = tous) ;
///   le worker lui redistribue ce qui la concerne, avec son numéro (`id`).
/// - Il porte un **petit état partagé** (dépôt courant, sélection, conversation ouverte, cartes
///   en attente…) : une page l'écrit (`etat-ecrire`), toutes le reçoivent (`etat`), une page
///   neuve le reçoit en entier à son branchement. C'est la forme qui laissera des fenêtres
///   détachées de l'IDE (le chat dans l'une, le graphe dans l'autre) se suivre sans rien refaire.
/// - Une page bat toutes les 15 s (`battement`) et prévient en partant (`partir`) ; le worker
///   oublie un port muet depuis 45 s, et ferme le flux quand plus aucune page n'écoute depuis
///   `GRACE_MS`. Le délai garde le même flux à travers une navigation dans un onglet (la page
///   quittée part, la neuve arrive) : fermé puis rouvert, l'ancien restait tenu par le
///   navigateur, et l'hôte comptait deux flux (mesuré le 13 novembre 2026). Le dernier onglet
///   fermé, le navigateur arrête le worker, et sa connexion avec lui.

const ADRESSE = "/ide/flux?sujets=files,dataflow,catalog,reactions,declarations,agent,tokens,repos,accords,etat,index,mailbox";
const MUET_MS = 45_000;
const GRACE_MS = 10_000;

type Statut = "ouvert" | "coupé, reprise…" | "fermé";
type Page = { port: MessagePort; sujets: Set<string> | null; vu: number; id: number; role?: string; backend?: string; focus: number };
let pagesNumerotees = 0;

const pages = new Set<Page>();
const etat = new Map<string, unknown>();
/// **The text of each turn in flight** (the IDE in several windows, 19 Nov. 2026): a window opened in the middle of
/// a turn asks for it (`tour`), and shows the turn's beginning, not only what streams after it opened. Kept from the
/// turn's tokens until its `RunFinished`, bounded at `tourMax` characters. Past the bound (ED's review), the text and
/// its `seq` stop together and `tronque` says it: the page marks the gap and keeps every token it received after it.
/// At most TOURS_MAX runs are kept, the oldest forgotten first, and all are forgotten on `EventsMissed` (a lost
/// `RunFinished` must not keep a text forever).
const tours = new Map<string, { texte: string; seq: number; tronque: boolean }>();
let tourMax = 512 * 1024;
const TOURS_MAX = 8;
let source: EventSource | null = null;
let dernier = "";
let statut: Statut = "fermé";

function diffuser(message: unknown) {
  for (const p of pages) p.port.postMessage(message);
}

function direLeStatut(s: Statut) {
  statut = s;
  diffuser({ type: "statut", statut: s });
}

function brancher() {
  const s = new EventSource(dernier ? `${ADRESSE}&depuis=${encodeURIComponent(dernier)}` : ADRESSE);
  s.onopen = () => direLeStatut("ouvert");
  s.onerror = () => direLeStatut("coupé, reprise…");
  s.onmessage = (e) => {
    if (e.lastEventId) dernier = e.lastEventId;
    let sujet: string | undefined;
    let ev: { kind?: string; run?: string; text?: string; seq?: number } | undefined;
    try {
      const j = JSON.parse(e.data);
      sujet = j?.sujet;
      ev = j?.evenement;
    } catch {
      /* une ligne illisible va à tous : la page la jettera */
    }
    if (ev?.run && ev.kind === "Token") {
      let t = tours.get(ev.run);
      if (!t) {
        if (tours.size >= TOURS_MAX) tours.delete(tours.keys().next().value as string);
        t = { texte: "", seq: -1, tronque: false };
        tours.set(ev.run, t);
      }
      const texte = String(ev.text ?? "");
      if (t.tronque || t.texte.length + texte.length > tourMax) t.tronque = true;
      else {
        t.texte += texte;
        if (typeof ev.seq === "number") t.seq = Math.max(t.seq, ev.seq);
      }
    } else if (ev?.run && ev.kind === "RunFinished") tours.delete(ev.run);
    else if (ev?.kind === "EventsMissed") tours.clear();
    for (const p of pages) {
      if (!p.sujets || sujet === undefined || p.sujets.has(sujet)) p.port.postMessage({ type: "evenement", data: e.data, id: e.lastEventId });
    }
  };
  source = s;
}

let fermeture: ReturnType<typeof setTimeout> | null = null;
function debrancherSiPersonne() {
  if (pages.size > 0 || !source || fermeture) return;
  fermeture = setTimeout(() => {
    fermeture = null;
    if (pages.size === 0 && source) {
      source.close();
      source = null;
      statut = "fermé";
    }
  }, GRACE_MS);
}

/// **The open windows of the IDE** (slice 2, ED's review: kept by the worker, never written by the pages, so that two
/// windows opening together do not lose one another): those that declared a role (`?fenetre=`), their backend and
/// when they last had the focus. Shared as the state key `fenetres`.
function publierLesFenetres() {
  const liste = [...pages].filter((p) => p.role).map((p) => ({ id: p.id, role: p.role, backend: p.backend, focus: p.focus }));
  etat.set("fenetres", liste);
  diffuser({ type: "etat", cle: "fenetres", valeur: liste });
}

function oublier(p: Page) {
  pages.delete(p);
  if (p.role) publierLesFenetres();
  debrancherSiPersonne();
}

(self as unknown as { onconnect: (e: MessageEvent) => void }).onconnect = (e: MessageEvent) => {
  const port = e.ports[0];
  const page: Page = { port, sujets: null, vu: Date.now(), id: ++pagesNumerotees, focus: 0 };
  pages.add(page);
  port.onmessage = (m: MessageEvent) => {
    const d = m.data ?? {};
    page.vu = Date.now();
    switch (d.type) {
      case "abonner":
        page.sujets = Array.isArray(d.sujets) && d.sujets.length ? new Set(d.sujets.map(String)) : null;
        break;
      case "partir":
        oublier(page);
        break;
      case "revenir":
        if (!pages.has(page)) pages.add(page);
        if (!source) brancher();
        port.postMessage({ type: "statut", statut });
        port.postMessage({ type: "etat-complet", etat: Object.fromEntries(etat) });
        break;
      case "tour":
        port.postMessage({ type: "tour", run: d.run, tour: tours.get(String(d.run)) ?? null });
        break;
      case "fenetre":
        page.role = String(d.role ?? "") || undefined;
        page.backend = d.backend ? String(d.backend) : undefined;
        page.focus = Date.now();
        port.postMessage({ type: "moi", id: page.id });
        publierLesFenetres();
        break;
      case "focus":
        page.focus = Date.now();
        if (page.role) publierLesFenetres();
        break;
      case "tour-max":
        // The witnesses only: a low bound, to prove what is said past it.
        if (typeof d.max === "number" && d.max > 0) tourMax = d.max;
        break;
      case "etat-ecrire":
        if (typeof d.cle === "string") {
          etat.set(d.cle, d.valeur);
          diffuser({ type: "etat", cle: d.cle, valeur: d.valeur });
        }
        break;
      default:
        /* battement : `vu` suffit */
        break;
    }
  };
  port.start();
  if (!source) brancher();
  port.postMessage({ type: "statut", statut });
  port.postMessage({ type: "etat-complet", etat: Object.fromEntries(etat) });
};

setInterval(() => {
  const limite = Date.now() - MUET_MS;
  for (const p of [...pages]) if (p.vu < limite) {
    pages.delete(p);
    if (p.role) publierLesFenetres();
  }
  debrancherSiPersonne();
}, 15_000);
