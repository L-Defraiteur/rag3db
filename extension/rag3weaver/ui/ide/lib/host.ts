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
};
export type HostState = { backends: Backend[]; sessions?: unknown[] };

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
  return r.result?.result ?? r.result;
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
  const source = new EventSource("/ide/flux?sujets=files,dataflow,catalog,reactions,declarations,agent,tokens");
  source.onopen = () => onStatus("ouvert");
  source.onerror = () => onStatus("coupé, reprise…");
  source.onmessage = (e) => {
    try {
      onEvent(JSON.parse(e.data));
    } catch {
      /* une ligne illisible ne coupe pas le flux */
    }
  };
  (window as unknown as { __ide?: { flux?: EventSource } }).__ide = { flux: source };
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

export type ChatTurn = {
  role: "user" | "assistant" | "tool" | "system";
  content?: string;
  tool_calls?: { id: string; name: string; arguments: string }[];
  tool_call_id?: string;
  tool_name?: string;
};
export type ChatState = { on: boolean; model: string | null; why: string | null; run: string | null; turns: ChatTurn[]; orphans: { pourquoi: string }[] };

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
