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
///   oublie un port muet depuis 45 s, et ferme le flux quand plus aucune page n'écoute.

const ADRESSE = "/ide/flux?sujets=files,dataflow,catalog,reactions,declarations,agent,tokens,repos,accords,etat";
const MUET_MS = 45_000;

type Statut = "ouvert" | "coupé, reprise…" | "fermé";
type Page = { port: MessagePort; sujets: Set<string> | null; vu: number };

const pages = new Set<Page>();
const etat = new Map<string, unknown>();
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
    try {
      sujet = JSON.parse(e.data)?.sujet;
    } catch {
      /* une ligne illisible va à tous : la page la jettera */
    }
    for (const p of pages) {
      if (!p.sujets || sujet === undefined || p.sujets.has(sujet)) p.port.postMessage({ type: "evenement", data: e.data, id: e.lastEventId });
    }
  };
  source = s;
}

function debrancherSiPersonne() {
  if (pages.size === 0 && source) {
    source.close();
    source = null;
    statut = "fermé";
  }
}

function oublier(p: Page) {
  pages.delete(p);
  debrancherSiPersonne();
}

(self as unknown as { onconnect: (e: MessageEvent) => void }).onconnect = (e: MessageEvent) => {
  const port = e.ports[0];
  const page: Page = { port, sujets: null, vu: Date.now() };
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
  for (const p of [...pages]) if (p.vu < limite) pages.delete(p);
  debrancherSiPersonne();
}, 15_000);
