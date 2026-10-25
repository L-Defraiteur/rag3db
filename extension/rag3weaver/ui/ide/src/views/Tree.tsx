import { useEffect, useState } from "react";
import { Folder, FileText } from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { call, shortName, type HostState } from "@/lib/host";
import { fileLink } from "@/views/File";

// **L'arbre d'un backend** (page 08, vue 7) : les dossiers puis les fichiers sous
// un préfixe, chacun avec ses lignes, « modifié depuis l'index » ou « non indexé ».
// Repère gardé : les `#vue li a` sont, dans l'ordre, les dossiers (`nom/`) puis
// les fichiers.
type Entry = { path: string; lines?: number | null; stale?: boolean; indexed?: boolean };
const enc = (s: string) => s.split("/").map(encodeURIComponent).join("/");

export function Tree({ id, dir, state }: { id: string; dir: string; state: HostState }) {
  const backend = state.backends.find((b) => b.id === id);
  const prefix = dir ? (dir.endsWith("/") ? dir : dir + "/") : "";
  const [r, setR] = useState<{ entries: Entry[]; total: number; returned: number } | null>(null);
  const [error, setError] = useState<string | null>(null);
  useEffect(() => {
    if (!backend) return;
    (call(backend.manifest, "list_files", { path_prefix: prefix, limit: 2000, format: "json" }) as Promise<{ entries: Entry[]; total: number; returned: number }>).then(setR).catch((e: Error) => setError(e.message));
  }, [backend?.manifest, prefix]);
  if (!backend) return <p className="text-destructive">backend inconnu : {id}</p>;
  const folders = new Map<string, number>();
  const files: Entry[] = [];
  for (const e of r?.entries ?? []) {
    const rest = e.path.slice(prefix.length);
    const [first, ...more] = rest.split("/");
    if (more.length) folders.set(first, (folders.get(first) ?? 0) + 1);
    else files.push(e);
  }
  const parts = prefix.split("/").filter(Boolean);
  return (
    <>
      <p id="fil" className="text-xs text-muted-foreground mb-3">
        <a href={`/ide/code/${id}/arbre/`} className="hover:underline">{shortName(backend.manifest)}</a>
        {parts.map((p, i) => <span key={i}>{" / "}<a href={`/ide/code/${id}/arbre/${enc(parts.slice(0, i + 1).join("/"))}/`} className="hover:underline">{p}</a></span>)}
      </p>
      <h1 className="text-xl font-semibold mb-2 font-mono">{prefix || "/"}</h1>
      {error ? <p className="text-destructive" role="alert">{error}</p> : null}
      {r ? (
        <>
          {r.total > r.returned ? <p className="note text-xs text-muted-foreground">{r.returned} fichiers montrés sur {r.total} sous ce préfixe.</p> : null}
          <ul className="liste grid gap-0.5 text-sm">
            {[...folders.entries()].sort().map(([d, n]) => (
              <li key={`d:${d}`} className="flex items-center gap-2">
                <Folder className="size-3.5 text-muted-foreground" aria-hidden />
                <a href={`/ide/code/${id}/arbre/${enc(prefix + d)}/`} className="hover:underline">{d + "/"}</a>
                <span className="meta text-xs text-muted-foreground">{n} fichier{n > 1 ? "s" : ""}</span>
              </li>
            ))}
            {files.sort((a, b) => a.path.localeCompare(b.path)).map((e) => (
              <li key={e.path} className="flex items-center gap-2">
                <FileText className="size-3.5 text-muted-foreground" aria-hidden />
                <a href={fileLink(id, e.path)} className="hover:underline">{e.path.slice(prefix.length)}</a>
                <span className="meta text-xs text-muted-foreground">{e.lines != null ? `${e.lines} lignes` : ""}</span>
                {e.stale ? <Badge variant="outline" className="pastille">modifié depuis l'index</Badge> : e.indexed === false ? <Badge variant="outline" className="pastille">non indexé</Badge> : null}
              </li>
            ))}
          </ul>
        </>
      ) : error ? null : (
        <p className="note text-muted-foreground">Chargement…</p>
      )}
    </>
  );
}
