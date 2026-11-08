import { useEffect, useState } from "react";
import { Badge } from "@/components/ui/badge";
import { call, shortName, type HostState } from "@/lib/host";
import { fileLink, symbolLink } from "@/views/File";

// **Un symbole** (page 08, vue 7) : ses définitions et ses usages par genre
// (`usages`), ce qui en dépend (`impact`), ce qu'il appelle (`callees`) — **chaque
// section se rend dès que sa réponse arrive**, « en cours » pour les autres (c7 :
// sous charge, un Promise.all laissait « Chargement… » tant que le plus lent
// n'avait pas répondu). Repères gardés : `#vue h2` « Définitions (n) », les liens
// `…/fichier/<chemin>#L<n>`, `.pastille` « par le nom ».
type Item = { path?: string; line?: number; title: string; kind?: string };
type Usages = { definitions?: Item[]; usages?: { usage: string; user: Item; guessed?: boolean }[]; ambiguous?: boolean };
type Reached = { reached?: Item[]; erreur?: string };
type Pending<T> = { state: "pending" } | { state: "done"; value: T } | { state: "error"; error: string };

export const graphLink = (id: string, name: string) => `/ide/code/${id}/graphe/${encodeURIComponent(name)}`;

export function ItemLink({ id, it, extra }: { id: string; it: Item; extra?: React.ReactNode }) {
  return (
    <li className="flex flex-wrap items-center gap-2">
      <a href={fileLink(id, it.path ?? "", it.line)} className="font-mono text-xs hover:underline">{it.path}{it.line ? ":" + it.line : ""}</a>
      <a href={symbolLink(id, it.title)} className="hover:underline">{it.title}</a>
      <span className="meta text-xs text-muted-foreground">{it.kind ?? ""}</span>
      {extra}
    </li>
  );
}

function use<T>(f: () => Promise<T>, deps: unknown[]): Pending<T> {
  const [s, set] = useState<Pending<T>>({ state: "pending" });
  useEffect(() => {
    set({ state: "pending" });
    f().then((value) => set({ state: "done", value })).catch((e: Error) => set({ state: "error", error: e.message }));
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, deps);
  return s;
}

function Neighbours({ id, title, r }: { id: string; title: string; r: Pending<Reached> }) {
  if (r.state === "pending") return <><h2 className="text-base font-semibold mt-4">{title}</h2><p className="note text-sm text-muted-foreground" data-en-cours>en cours…</p></>;
  if (r.state === "error" || r.value.erreur) return <><h2 className="text-base font-semibold mt-4">{title}</h2><p className="note text-sm text-muted-foreground">{r.state === "error" ? r.error : r.value.erreur}</p></>;
  const list = r.value.reached ?? [];
  return (
    <>
      <h2 className="text-base font-semibold mt-4">{title} ({list.length})</h2>
      <ul className="liste grid gap-0.5 text-sm">{list.map((it, k) => <ItemLink key={k} id={id} it={it} />)}</ul>
    </>
  );
}

export function Symbol({ id, name, state }: { id: string; name: string; state: HostState }) {
  const backend = state.backends.find((b) => b.id === id);
  const m = backend?.manifest ?? "";
  const u = use(() => call(m, "usages", { name, limit: 200, format: "json" }) as Promise<Usages>, [m, name]);
  const imp = use(() => (call(m, "impact", { name, format: "json" }) as Promise<Reached>).catch((e: Error) => ({ erreur: e.message })), [m, name]);
  const app = use(() => (call(m, "callees", { name, format: "json" }) as Promise<Reached>).catch((e: Error) => ({ erreur: e.message })), [m, name]);
  if (!backend) return <p className="text-destructive">backend inconnu : {id}</p>;
  const byKind = new Map<string, { user: Item; guessed?: boolean }[]>();
  if (u.state === "done") for (const x of u.value.usages ?? []) (byKind.get(x.usage) ?? byKind.set(x.usage, []).get(x.usage)!).push(x);
  return (
    <>
      <p id="fil" className="text-xs text-muted-foreground mb-3">
        <a href={`/ide/code/${id}/arbre/`} className="hover:underline">{shortName(backend.manifest)}</a> · {name}
      </p>
      <h1 className="text-xl font-semibold mb-2 flex items-center gap-2">
        {name} <a href={graphLink(id, name)} className="pastille text-xs font-normal underline">en graphe</a>
      </h1>
      {u.state === "pending" ? <p className="note text-sm text-muted-foreground" data-en-cours>Définitions et usages : en cours…</p> : null}
      {u.state === "error" ? <p className="text-destructive" role="alert">{u.error}</p> : null}
      {u.state === "done" ? (
        <>
          {u.value.ambiguous ? <p className="note text-sm text-muted-foreground">Nom défini plusieurs fois : les usages trouvés par le nom seul ne sont pas attribués.</p> : null}
          <h2 className="text-base font-semibold">Définitions ({(u.value.definitions ?? []).length})</h2>
          <ul className="liste grid gap-0.5 text-sm">{(u.value.definitions ?? []).map((d, k) => <ItemLink key={k} id={id} it={d} />)}</ul>
          {[...byKind.entries()].map(([kind, xs]) => (
            <div key={kind}>
              <h2 className="text-base font-semibold mt-4">{kind} ({xs.length})</h2>
              <ul className="liste grid gap-0.5 text-sm">{xs.map((x, k) => <ItemLink key={k} id={id} it={x.user} extra={x.guessed ? <Badge variant="outline" className="pastille">par le nom</Badge> : null} />)}</ul>
            </div>
          ))}
        </>
      ) : null}
      <Neighbours id={id} title="Ce qui en dépend (impact)" r={imp} />
      <Neighbours id={id} title="Ce qu'il appelle" r={app} />
    </>
  );
}
