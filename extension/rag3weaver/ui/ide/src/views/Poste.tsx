import { useEffect, useState } from "react";
import { Bot, Eye, FolderTree, GitBranch, ListChecks, Server } from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { fetchPoste, type Poste as PosteState } from "@/lib/host";

// **The machine at a glance** (Lucie, 20 November 2026: « gestion de tous les agents actifs ou non, savoir tous les
// dossiers sur le PC ouverts dans rag3weaver »; scenario 1 of the night). Read from the host's `GET /poste` (by
// `/ide/poste/etat`), every few seconds. An agent is a live session of the host (by its Claude name when its client
// gave it), or a session named in the table of the folders without a live session: inactive. Markers of the witness:
// `#agents`, `#dossiers-du-poste`, `#vues-ouvertes`, `#depots-du-poste`, `#disponibles-du-poste`, `#taches-du-poste`.
export function Poste() {
  const [poste, setPoste] = useState<PosteState | null>(null);
  const [error, setError] = useState<string | null>(null);
  useEffect(() => {
    let vivant = true;
    const lire = () =>
      fetchPoste()
        .then((p) => vivant && (setPoste(p), setError(null)))
        .catch((e: Error) => vivant && setError(e.message));
    lire();
    const t = setInterval(lire, 5000);
    return () => {
      vivant = false;
      clearInterval(t);
    };
  }, []);
  if (error && !poste) return <p className="text-destructive" role="alert">{error}</p>;
  if (!poste) return <p className="text-muted-foreground">le poste se lit…</p>;

  const quand = (ms?: number | null) => (ms ? new Date(ms).toLocaleString("fr-FR", { dateStyle: "short", timeStyle: "short" }) : "");
  const vivants = poste.sessions;
  const nomsVivants = new Set(vivants.map((s) => s.claude?.name).filter(Boolean) as string[]);
  const inactifs = [...new Set(poste.folders.map((f) => f.session))].filter((n) => n && !nomsVivants.has(n));
  const nomDe = (id: string) => vivants.find((s) => s.id === id || s.claude?.session_id === id)?.claude?.name ?? id;

  return (
    <>
      <p id="fil" className="text-xs text-muted-foreground mb-3">
        <a href="/ide/" className="hover:underline">dépôts</a> · poste {poste.machine}
      </p>
      <h1 className="text-xl font-semibold mb-4 flex items-center gap-2">
        <Server className="size-5" /> Poste {poste.machine}
      </h1>
      {error ? <p className="text-destructive text-sm mb-3">{error} (dernier état gardé)</p> : null}
      <div className="grid gap-4">
        <Section id="agents" icon={<Bot className="size-4" />} title={`Agents (${vivants.length} actifs, ${inactifs.length} inactifs)`}>
          <ul className="space-y-1 text-sm">
            {vivants.map((s) => (
              <li key={s.id} className="flex flex-wrap items-center gap-2">
                <Badge>actif</Badge>
                <span className="font-medium">{s.claude?.name ?? s.client}</span>
                <span className="text-muted-foreground">
                  {s.client}
                  {s.pid ? ` · pid ${s.pid}` : ""} · depuis {quand(s.opened_at)} · vue {s.active_view.name}
                  {s.active_view.source === "default" ? " (par défaut)" : ""}
                </span>
              </li>
            ))}
            {inactifs.map((n) => (
              <li key={n} className="flex flex-wrap items-center gap-2">
                <Badge variant="secondary">inactif</Badge>
                <span className="font-medium">{n}</span>
                <span className="text-muted-foreground">sans session ouverte ; ses dossiers restent inscrits</span>
              </li>
            ))}
            {!vivants.length && !inactifs.length ? <li className="text-muted-foreground">aucun agent</li> : null}
          </ul>
        </Section>

        <Section id="dossiers-du-poste" icon={<FolderTree className="size-4" />} title="Dossiers du poste">
          <ul className="space-y-2 text-sm">
            {poste.folders.map((f) => (
              <li key={f.folder}>
                <span className="font-mono">{f.folder}/</span> · {f.session}
                {f.exists ? "" : <Badge variant="outline" className="ml-2">absent du disque</Badge>}
                {f.worktrees.length ? (
                  <ul className="ml-4 text-xs text-muted-foreground font-mono">
                    {f.worktrees.map((w) => (
                      <li key={w} title={w}>{w.split("/").pop()}</li>
                    ))}
                  </ul>
                ) : null}
              </li>
            ))}
            {!poste.folders.length ? <li className="text-muted-foreground">aucun dossier inscrit</li> : null}
          </ul>
        </Section>

        <Section id="vues-ouvertes" icon={<Eye className="size-4" />} title="Vues ouvertes">
          <ul className="space-y-1 text-sm">
            {poste.open_views.map((v) => (
              <li key={`${v.session_id}/${v.name}`} title={v.path ?? undefined}>
                <span className="font-medium">{v.name}</span> · {v.branch ?? "?"} · {nomDe(v.session_id)}
                {v.active ? <Badge className="ml-2">active</Badge> : null}
                {v.writing ? <Badge variant="outline" className="ml-2">couche en écriture</Badge> : null}
              </li>
            ))}
            {!poste.open_views.length ? <li className="text-muted-foreground">aucune vue ouverte</li> : null}
          </ul>
        </Section>

        <Section id="depots-du-poste" icon={<GitBranch className="size-4" />} title="Dépôts attachés">
          <ul className="space-y-1 text-sm">
            {poste.repositories.map((r) => (
              <li key={r.id}>
                <a href={`/ide/code/${r.id}/arbre/`} className="font-medium hover:underline">{r.repo ?? r.name ?? r.id}</a>
                {r.root ? <span className="text-muted-foreground font-mono text-xs"> · {r.root}</span> : null}
              </li>
            ))}
            {!poste.repositories.length ? <li className="text-muted-foreground">aucun dépôt attaché</li> : null}
          </ul>
          {poste.available?.length ? (
            <div id="disponibles-du-poste" className="mt-3 text-sm">
              <p className="text-xs text-muted-foreground uppercase">disponibles</p>
              <ul>
                {poste.available.map((a) => (
                  <li key={a.backend}>{a.repo ?? a.backend}{a.branch ? ` · ${a.branch}` : ""}{a.path ? <span className="text-muted-foreground font-mono text-xs"> · {a.path}</span> : null}</li>
                ))}
              </ul>
            </div>
          ) : null}
        </Section>

        <Section id="taches-du-poste" icon={<ListChecks className="size-4" />} title="Tâches">
          <ul className="space-y-1 text-sm">
            {poste.tasks.map((t) => (
              <li key={t.id}>
                <a href={`/ide/code/${t.session}/taches/${encodeURIComponent(t.name)}`} className="font-medium hover:underline">{t.name}</a>
                {t.state ? <span className="ml-2 text-xs text-muted-foreground">{t.state}</span> : null}
                {t.awaits_answer ? <Badge className="ml-2">attend une réponse</Badge> : null}
                {(t.spaces ?? []).map((s) => (
                  <span key={s.backend} className="ml-2 text-xs font-mono text-muted-foreground">{s.branch ?? s.backend}</span>
                ))}
              </li>
            ))}
            {!poste.tasks.length ? <li className="text-muted-foreground">aucune tâche</li> : null}
          </ul>
        </Section>
      </div>
    </>
  );
}

function Section({ id, icon, title, children }: { id: string; icon: React.ReactNode; title: string; children: React.ReactNode }) {
  return (
    <Card id={id}>
      <CardHeader className="pb-2">
        <CardTitle className="flex items-center gap-2 text-base">{icon} {title}</CardTitle>
      </CardHeader>
      <CardContent>{children}</CardContent>
    </Card>
  );
}
