// **L'hôte, vu de la page** : son état (`/ide/etat`), un appel d'outil (`/ide/call`)
// et le flux SSE (`/ide/flux`), ouvert une fois et partagé par les vues. Les
// points d'entrée HTTP sont ceux de l'ancienne page : rien ne change côté hôte.
export type Task = { nom: string; parent: string; branche: string };
export type Backend = {
  id: string;
  manifest: string;
  open: boolean;
  tache: Task | null;
  template_drift?: string[];
  sessions?: number;
  // Ce que les gens nomment : le dépôt git qui contient le manifeste, sa branche,
  // et ce qu'est le backend — `code` (un dépôt), `memory` (la mémoire), `project`
  // (un projet rag3weaver déclaratif sans dépôt). « backend » ne paraît pas à l'écran.
  repo?: string | null;
  branch?: string | null;
  kind?: "code" | "memory" | "project";
  // `manifest` : la nature que le gabarit déclare ; `fallback` : l'hôte l'a devinée
  // (un espace de travail, un dépôt), le manifeste ne la dit pas.
  kind_source?: "manifest" | "fallback";
};

/// Le nom qu'une personne donne à ce que tient un backend : la branche d'une tâche,
/// « la mémoire », le dépôt (et sa branche), ou le nom d'un projet déclaratif.
export function displayName(b: Backend): string {
  if (b.tache) return b.tache.branche;
  if (b.kind === "memory") return "la mémoire";
  if (b.kind === "code" && b.repo) return b.branch ? `${b.repo} · ${b.branch}` : b.repo;
  return shortName(b.manifest).split("/").pop() || shortName(b.manifest);
}
// **La barre du chat** (modèle tranché par c7 avec Lucie, forme accordée avec
// everything declarative, page 02) : des listes à plat. Une session (un backend
// principal) tient son espace ; ses tâches (`session`) tiennent un à n espaces
// (un worktree par dépôt touché), une proposition par espace ; une sous-tâche
// dit sa tâche mère (`parent_task`).
export type Rag = { state: "indexed" | "indexing" | "absent"; percent?: number | null };
export type Space = {
  backend: string;
  repo?: string | null;
  branch?: string | null;
  path?: string | null;
  loaded: boolean;
  rag: Rag;
  proposal?: { state: "open" | "merged" | "rejected" } | null;
};
export type RunState = "running" | "waiting" | "ended" | "error";
export type ChatSession = {
  id: string;
  name?: string | null;
  model?: string | null;
  state?: RunState | null;
  awaits_answer?: boolean;
  memory: { backend: string | null; rag: boolean };
  spaces: Space[];
};
export type SessionTask = { id: string; session: string; parent_task?: string | null; name: string; state?: RunState | null; awaits_answer?: boolean; spaces: Space[] };
export type HostState = { backends: Backend[]; sessions?: unknown[]; chat_sessions?: ChatSession[]; tasks?: SessionTask[]; available_repos?: Space[] };

export async function fetchState(): Promise<HostState> {
  const r = await fetch("/ide/etat");
  if (!r.ok) throw new Error(`/ide/etat : HTTP ${r.status}`);
  return r.json();
}

export async function call(manifest: string, name: string, args: unknown): Promise<unknown> {
  const r = await (await fetch("/ide/call", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ manifest, name, arguments: args }),
  })).json();
  if (!r.ok) throw new Error(r.error ?? "appel refusé");
  // La sortie de l'outil, un cran sous la réponse du graphe ({ result, metadata, … }),
  // et sous l'enveloppe { status, result } d'une lecture du catalogue quand il y en a
  // une (usages, impact, callees… : `status` dit l'état de l'index) — comme
  // l'ancienne page le faisait ; un `result` nul est l'erreur que `status` dit.
  return unwrap(r.result?.result ?? r.result).value;
}

/// Le nom court d'un backend : les deux derniers dossiers de son manifeste.
export function shortName(manifest: string): string {
  return manifest.split("/").slice(-3, -1).join("/") || manifest;
}

export type FluxEvent = { sujet: string; backend: string; at_ms?: number; evenement?: Record<string, unknown> };
export type FluxStatus = "ouvert" | "coupé, reprise…" | "fermé";

/// Le flux de l'hôte : un `EventSource` pour toute la page. `Last-Event-ID` est
/// renvoyé par le navigateur à la reconnexion ; l'hôte rejoue ce qu'il garde.
export function openFlux(onEvent: (e: FluxEvent) => void, onStatus: (s: FluxStatus) => void): () => void {
  // `repos` : un worktree attaché par l'atelier (`RepoAttached`), que la barre montre.
  const source = new EventSource("/ide/flux?sujets=files,dataflow,catalog,reactions,declarations,agent,tokens,repos,accords");
  source.onopen = () => onStatus("ouvert");
  source.onerror = () => onStatus("coupé, reprise…");
  source.onmessage = (e) => {
    try {
      onEvent(JSON.parse(e.data));
    } catch {
      /* une ligne illisible ne coupe pas le flux */
    }
  };
  // Le reste de `__ide` (les relevés des vues, que les témoins lisent) est gardé.
  const w = window as unknown as { __ide?: Record<string, unknown> };
  w.__ide = { ...(w.__ide ?? {}), flux: source };
  return () => source.close();
}

// **Le bus du flux dans la page** : une vue s'abonne aux événements que le
// cadre reçoit (un seul `EventSource` pour toute la page).
type Listener = (e: FluxEvent) => void;
const listeners = new Set<Listener>();
export function onFlux(l: Listener): () => void {
  listeners.add(l);
  return () => listeners.delete(l);
}
export function dispatchFlux(e: FluxEvent) {
  for (const l of listeners) l(e);
}

// Un tour de la session. `at_ms` : sur un appel, l'heure où il est parti ; sur un
// tour `tool`, l'heure où le résultat est revenu (la durée est l'écart) ; absent
// des tours d'avant.
export type ChatTurn = {
  role: "user" | "assistant" | "tool" | "system";
  content?: string;
  tool_calls?: { id: string; name: string; arguments: string; at_ms?: number }[];
  tool_call_id?: string;
  tool_name?: string;
  at_ms?: number;
  /// Les composants qu'un résultat d'outil porte pour la page, hors du texte du modèle.
  ui?: UiComposant[];
};
/// Un composant `ui` d'un résultat d'outil : son nom (`kind`), les sujets du flux qu'il
/// écoute, et ce qu'il montre (pour `index_progress` : le dépôt et son manifeste, la clé
/// du flux).
export type UiComposant = { kind: string; sujets?: string[]; repo?: string; manifest?: string; titre?: string; id?: string; chemin?: string; mode?: string; raison?: string; genre?: string; commande?: string };
// La jauge du contexte : `used`, les jetons d'entrée (cache compris) du dernier
// appel au modèle ; `max`, le contexte du modèle déclaré (null : inconnu).
export type ChatContext = { used: number | null; max: number | null; source: string; at: number | null };
// Une fiche de la mémoire déclarée écrite pendant cette conversation : par une
// Source dont `session` est un run de la conversation (le juge), ou un apport qui
// la cite. Les écritures de l'agent lui-même attendent M7 (la session posée par
// l'hôte).
export type MemoryWrite = { fiche: string; kind: "fiche" | "apport"; section?: string; at_ms?: number; run: string };
// L'usage du modèle, lu au journal des runs du chat : le dernier tour et la
// conversation, par unité du compteur (input_tokens, cached_input_tokens,
// output_tokens, reasoning_tokens).
export type ChatUsage = { tour: Record<string, number> | null; tour_run: string | null; session: Record<string, number>; tours: number; lus_max?: number };
// Une tâche à rôle finie, lue au journal (la page ajoute celles en cours par le flux).
export type RoleRunDone = { role: string; run: string; tool?: string | null; reaction?: string | null; state: "fini" | "échec" | "inconnu"; at_ms?: number | null };
export type ChatState = {
  on: boolean;
  model: string | null;
  context?: ChatContext;
  memory_writes?: MemoryWrite[];
  // Pourquoi la liste des fiches écrites ne peut pas se faire (pas de mémoire
  // déclarée, une mémoire sans outil de lecture par filtre…) ; null sinon.
  memory_writes_why?: string | null;
  usage?: ChatUsage;
  role_runs?: RoleRunDone[];
  why: string | null;
  run: string | null;
  turns: ChatTurn[];
  orphans: { pourquoi: string }[];
};

export async function fetchChat(backend: string): Promise<ChatState> {
  const r = await (await fetch(`/ide/chat?${new URLSearchParams({ backend })}`)).json();
  if (!r.ok) throw new Error(r.error);
  return r.result;
}

export type TurnResult = {
  run: string;
  text: string;
  stopped_by: string | null;
  consumed: { slug: string; amount: number; unit: string }[];
  propositions: { id: number; paths?: string[]; accepted?: boolean }[];
  tache: Task | null;
  fichiers: string[];
};

export async function sendChat(backend: string, message: string): Promise<TurnResult> {
  const r = await (await fetch("/ide/chat", { method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify({ backend, message }) })).json();
  if (!r.ok) throw new Error(r.error);
  return r.result;
}

export async function stopRun(backend: string, run: string): Promise<void> {
  const r = await (await fetch("/ide/runs/arreter", { method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify({ backend, run }) })).json();
  if (!r.ok) throw new Error(r.error);
}

/// L'URL de la page lue comme une route : l'accueil, ou une vue d'un backend.
export function route(): { view: string; id?: string; rest?: string } {
  const p = location.pathname.replace(/\/+$/, "") || "/ide";
  const m = p.match(/^\/ide\/code\/([^/]+)\/([a-z]+)(?:\/(.*))?$/);
  if (!m) return { view: "home" };
  return { view: m[2], id: m[1], rest: decodeURIComponent(m[3] ?? "") };
}

/// La réponse entière du graphe d'un outil ({ result, metadata, freshness, … }) :
/// pour une vue qui montre aussi ce que l'hôte dit à côté de la sortie.
export async function callFull(manifest: string, name: string, args: unknown): Promise<Record<string, unknown>> {
  const r = await (await fetch("/ide/call", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ manifest, name, arguments: args }),
  })).json();
  if (!r.ok) throw new Error(r.error ?? `${name} : échec`);
  return r.result ?? {};
}

/// La sortie d'un outil sous l'enveloppe d'une lecture du catalogue ({ status, result })
/// s'il y en a une ; un `result` nul est l'erreur que `status` dit.
export function unwrap(s: unknown): { value: unknown; status?: string } {
  if (!s || typeof s !== "object" || Array.isArray(s)) return { value: s };
  const o = s as Record<string, unknown>;
  const keys = Object.keys(o);
  if (keys.length !== 2 || !keys.includes("status") || !keys.includes("result")) return { value: s };
  if (o.result === null) throw new Error(String(o.status));
  return { value: o.result, status: String(o.status) };
}
