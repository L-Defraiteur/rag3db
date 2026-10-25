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
