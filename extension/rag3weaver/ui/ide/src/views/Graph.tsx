import { useEffect, useState } from "react";
import { call, shortName, type HostState } from "@/lib/host";
import { fileLink, symbolLink } from "@/views/File";
import { graphLink } from "@/views/Symbol";
import { short } from "@/lib/graph";

// **Le voisinage en papillon** (page 08, vue 7) : ce qui en dépend à gauche
// (impact), ce qu'il appelle à droite (callees), une colonne par niveau ; une
// arête suit `from`, le nœud d'où le parcours l'a atteint ; un nœud sans `from`
// (atteint par le nom seul) se rattache au centre, en pointillé : « peut-être ».
// Repères gardés : `#vue svg g.noeud`, `g.noeud.centre`.
type Item = { uuid: string; title: string; path?: string; line?: number; level?: number; from?: string; by_name?: boolean; relation?: string };
type Side = { starts?: Item[]; reached?: Item[] };

export function GraphView({ id, name, state }: { id: string; name: string; state: HostState }) {
  const backend = state.backends.find((b) => b.id === id);
  const [imp, setImp] = useState<Side | null>(null);
  const [app, setApp] = useState<Side | null>(null);
  useEffect(() => {
    if (!backend) return;
    (call(backend.manifest, "impact", { name, depth: 3, format: "json" }) as Promise<Side>).catch(() => ({ starts: [], reached: [] })).then(setImp);
    (call(backend.manifest, "callees", { name, depth: 3, format: "json" }) as Promise<Side>).catch(() => ({ starts: [], reached: [] })).then(setApp);
  }, [backend?.manifest, name]);
  if (!backend) return <p className="text-destructive">backend inconnu : {id}</p>;
  if (!imp || !app) return <p className="note text-muted-foreground">Chargement…</p>;
  const L = 210, H = 34, W = 180, M = 24;
  const centres = [...(imp.starts ?? []), ...(app.starts ?? [])].filter((d, i, t) => t.findIndex((x) => x.uuid === d.uuid) === i);
  const sides: { r: Side; sens: number }[] = [{ r: imp, sens: -1 }, { r: app, sens: 1 }];
  const columns = new Map<string, Item[]>();
  for (const { r, sens } of sides) for (const x of r.reached ?? []) { const k = `${sens}:${x.level}`; (columns.get(k) ?? columns.set(k, []).get(k)!).push(x); }
  const depth = Math.max(1, ...[...columns.keys()].map((k) => Number(k.split(":")[1])));
  const height = Math.max(centres.length, ...[...columns.values()].map((c) => c.length), 1) * H + 2 * M;
  const width = (2 * depth + 1) * L + 2 * M;
  const pos = new Map<string, { x: number; y: number; x0: Item }>();
  const place = (key: (x: Item) => string, items: Item[], col: number) => items.forEach((x, i) => pos.set(key(x), { x: M + col * L, y: M + (height - 2 * M - items.length * H) / 2 + i * H, x0: x }));
  place((x) => `c:${x.uuid}`, centres, depth);
  for (const [k, items] of columns) { const [sens, level] = k.split(":").map(Number); place((x) => `${sens}:${x.uuid}`, items, depth + sens * level); }
  const where = (sens: number, uuid: string | undefined) => pos.get(`${sens}:${uuid}`) ?? pos.get(`c:${uuid}`) ?? pos.get(`c:${centres[0]?.uuid}`);
  const edges: React.ReactNode[] = [];
  for (const { r, sens } of sides) for (const x of r.reached ?? []) {
    const a = pos.get(`${sens}:${x.uuid}`);
    const b = x.from ? where(sens, x.from) : where(sens, centres[0]?.uuid);
    if (!a || !b) continue;
    const [g, d] = sens < 0 ? [a, b] : [b, a];
    const x1 = g.x + W, y1 = g.y + H / 2 - 4, x2 = d.x, y2 = d.y + H / 2 - 4;
    edges.push(<path key={`${sens}:${x.uuid}`} d={`M${x1},${y1} C${(x1 + x2) / 2},${y1} ${(x1 + x2) / 2},${y2} ${x2},${y2}`} className={x.from && !x.by_name ? "" : "devine"} fill="none" stroke="currentColor" strokeOpacity={0.5} strokeDasharray={x.from && !x.by_name ? undefined : "4 3"} />);
  }
  const total = (imp.reached ?? []).length + (app.reached ?? []).length;
  return (
    <>
      <p id="fil" className="text-xs text-muted-foreground mb-3"><a href={`/ide/code/${id}/arbre/`} className="hover:underline">{shortName(backend.manifest)}</a> · <a href={symbolLink(id, name)} className="hover:underline">{name}</a> · graphe</p>
      <h1 className="text-xl font-semibold mb-1">{name}</h1>
      <p className="note text-sm text-muted-foreground mb-3">{(imp.reached ?? []).length} en dépendent (à gauche), {(app.reached ?? []).length} appelés (à droite), sur trois niveaux. Un nom mène à son propre graphe, un lieu à son fichier ; un trait pointillé est un rattachement par le nom seul.</p>
      {total || centres.length ? (
        <div className="graphe overflow-x-auto">
          <svg width={width} height={height} viewBox={`0 0 ${width} ${height}`} className="text-foreground">
            <g>{edges}</g>
            <g>
              {[...pos.entries()].map(([key, p]) => {
                const x = p.x0;
                const centre = key.startsWith("c:");
                const lieu = `${x.path ?? ""}${x.line ? ":" + x.line : ""}`;
                return (
                  <g key={key} className={`noeud${centre ? " centre" : ""}`} transform={`translate(${p.x},${p.y})`}>
                    <title>{`${x.title} — ${lieu}${x.relation ? " — " + x.relation : ""}`}</title>
                    <rect width={W} height={H - 8} rx={5} className={centre ? "fill-accent stroke-primary" : "fill-card stroke-border"} strokeWidth={centre ? 1.5 : 1} />
                    <a href={centre ? symbolLink(id, x.title) : graphLink(id, x.title)}><text x={8} y={12} className="titre" fontSize={11} fontWeight={600} fill="currentColor">{short(x.title, 26)}</text></a>
                    <a href={fileLink(id, x.path ?? "", x.line)}><text x={8} y={23} className="lieu" fontSize={9} fill="currentColor" opacity={0.7}>{short(lieu, 32)}</text></a>
                  </g>
                );
              })}
            </g>
          </svg>
        </div>
      ) : (
        <p className="note text-sm text-muted-foreground">Rien d'atteint depuis ce nom.</p>
      )}
    </>
  );
}
