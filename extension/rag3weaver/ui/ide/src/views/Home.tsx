import { FolderOpen, FolderTree, Server, GitBranch, History, MessageSquare, Scale, Workflow, Zap, FileDiff } from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { displayName, type Backend, type HostState } from "@/lib/host";
import { SearchBox } from "@/components/SearchBox";
import { Button } from "@/components/ui/button";
import { useState } from "react";

// **L'accueil** : les dépôts de l'hôte, chacun avec ses vues et ses tâches (Lucie, 8
// novembre 2026 : l'accueil parle en dépôts, comme la barre du chat). Une carte se nomme
// par son dépôt et sa branche (`displayName`), le chemin du manifeste n'est qu'en
// infobulle, une tâche se dit par son worktree (sa branche `agent/<nom>`) ; « backend »
// ne paraît pas à l'écran. Repères des témoins : `#vue h1` « Dépôts », `.depot[data-backend]`
// et son `.nom`, `.worktree`, un lien `/ide/code/<id>/arbre/` par dépôt.
export function Home({ state }: { state: HostState }) {
  const backends = state.backends ?? [];
  if (!backends.length) {
    // **How to open a project** (scenario 2 of the night of 20 November 2026): a newcomer who reads only the IDE
    // stopped at this page. It gives the command and the folder to run it from, and what follows.
    return (
      <>
      <OpenFolder />
      <div className="accueil-vide text-muted-foreground space-y-2 mt-4">
        <p className="mb-4 text-sm"><a href="/ide/poste" className="lien-poste inline-flex items-center gap-1 hover:underline"><Server className="size-4" /> Poste : les agents, les dossiers, les vues ouvertes</a></p>
        <p>L'hôte ne sert encore aucun dépôt.</p>
        <p>
          Pour ouvrir un projet, lancez dans un terminal, depuis le dossier du projet :{" "}
          <code className="text-foreground">rag3weaver-backend atelier</code>
        </p>
        <p>
          L'atelier paraît alors ici. Son chat crée les fichiers et le dépôt git (sans rien pousser), ou attache un
          worktree déjà sur le disque.
        </p>
      </div>
      </>
    );
  }
  const principals = backends.filter((b) => !b.tache);
  const tasksOf = (b: Backend) => backends.filter((t) => t.tache && t.tache.parent === b.id);
  return (
    <>
      <OpenFolder />
      <p className="mb-4 text-sm"><a href="/ide/poste" className="lien-poste inline-flex items-center gap-1 hover:underline"><Server className="size-4" /> Poste : les agents, les dossiers, les vues ouvertes</a></p>
      <h1 className="text-xl font-semibold mb-4 mt-6">Dépôts</h1>
      <ul id="dossiers-ouverts" className="liste grid gap-3">
        {principals.map((b) => (
          <li key={b.id} className="depot dossier" data-backend={b.id} title={b.repo ?? undefined}>
            <Card>
              <CardHeader className="pb-2">
                <CardTitle className="flex items-center gap-2 text-base">
                  <a href={`/ide/code/${b.id}/arbre/`} className="nom hover:underline" title={b.manifest}>
                    {displayName(b)}
                  </a>
                  <Badge variant={b.open ? "default" : "secondary"}>{b.open ? "chargé" : "non chargé"}</Badge>
                  {(b.template_drift ?? []).length ? (
                    <Badge variant="outline" title={b.template_drift!.join("\n")}>
                      {b.template_drift!.length} gabarit(s) local(aux) différent(s) du livré
                    </Badge>
                  ) : null}
                </CardTitle>
              </CardHeader>
              <CardContent className="flex flex-wrap gap-x-4 gap-y-1 text-sm">
                <div className="basis-full">
                  <SearchBox id={b.id} domId={principals.length === 1 ? "chercher" : `chercher-${b.id}`} />
                </div>
                <Link href={`/ide/code/${b.id}/dag/`} icon={<Workflow className="size-4" />} label="graphes" />
                <Link href={`/ide/code/${b.id}/reactions/`} icon={<Zap className="size-4" />} label="réactions" />
                <Link href={`/ide/code/${b.id}/propositions/`} icon={<FileDiff className="size-4" />} label="propositions" />
                <Link href={`/ide/code/${b.id}/taches/`} icon={<GitBranch className="size-4" />} label="tâches" />
                <Link href={`/ide/code/${b.id}/chat`} icon={<MessageSquare className="size-4" />} label="chat" />
                <Link href={`/ide/code/${b.id}/bilans`} icon={<Scale className="size-4" />} label="bilans" />
                <Link href={`/ide/code/${b.id}/recherches/`} icon={<History className="size-4" />} label="mes recherches" />
                {tasksOf(b).map((t) => (
                  <span key={t.id} className="flex items-center gap-1 basis-full pt-1">
                    <Badge variant="outline" className="pastille worktree" title={t.manifest}>
                      worktree {displayName(t)} · tâche {t.tache!.nom}
                    </Badge>
                    <Link href={`/ide/code/${t.id}/arbre/`} icon={<FolderTree className="size-4" />} label="son arbre" />
                    <Link href={`/ide/code/${t.id}/chat`} icon={<MessageSquare className="size-4" />} label="chat" />
                    <Link href={`/ide/code/${t.id}/bilans`} icon={<Scale className="size-4" />} label="bilans" />
                    <Link href={`/ide/code/${b.id}/taches/${encodeURIComponent(t.tache!.nom)}`} icon={<FileDiff className="size-4" />} label="relecture" />
                  </span>
                ))}
              </CardContent>
            </Card>
          </li>
        ))}
      </ul>
    </>
  );
}

function Link({ href, icon, label }: { href: string; icon: React.ReactNode; label: string }) {
  return (
    <a href={href} className="inline-flex items-center gap-1 text-foreground/80 hover:text-foreground hover:underline">
      {icon} {label}
    </a>
  );
}

type Opened = { path: string; created: boolean };

// **Open a folder** (Lucie's first step in the IDE, 21 November 2026): an absolute path; a new or empty folder is
// created and gets `git init` (never a remote, never a push), a git repository is attached to the atelier; refused,
// the host says why. The folder then appears among the repositories (the host pushes its state). Markers of the
// witness: `#ouvrir input[name=chemin]`, `#ouvrir .ouvrir-resultat`, `#dossiers-ouverts .dossier`.
function OpenFolder() {
  const [path, setPath] = useState("");
  const [busy, setBusy] = useState(false);
  const [said, setSaid] = useState<{ ok: boolean; text: string } | null>(null);
  const submit = async (e: React.FormEvent) => {
    e.preventDefault();
    setBusy(true);
    setSaid(null);
    try {
      const r = await (await fetch("/ide/ouvrir", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ path: path.trim() }),
      })).json();
      if (r.ok) {
        const o: Opened = r.result;
        setSaid({
          ok: true,
          text: o.created
            ? `${o.path} : dossier créé, git init fait (aucun distant, rien de poussé), attaché à l'atelier.`
            : `${o.path} : dépôt git attaché à l'atelier.`,
        });
      } else {
        setSaid({ ok: false, text: r.error ?? "refusé" });
      }
    } catch (err) {
      setSaid({ ok: false, text: (err as Error).message });
    } finally {
      setBusy(false);
    }
  };
  return (
    <form id="ouvrir" onSubmit={submit} className="space-y-2">
      <label className="flex items-center gap-2 font-medium" htmlFor="ouvrir-chemin">
        <FolderOpen className="size-4" /> Ouvrir un dossier
      </label>
      <div className="flex gap-2">
        <input
          id="ouvrir-chemin"
          name="chemin"
          value={path}
          onChange={(e) => setPath(e.target.value)}
          placeholder="/home/vous/projets/mon-projet (absent ou vide : créé, avec git init)"
          className="flex-1 rounded-md border bg-background px-3 py-1.5 text-sm font-mono"
        />
        <Button type="submit" disabled={busy || !path.trim()}>
          {busy ? "Ouverture…" : "Ouvrir"}
        </Button>
      </div>
      {said ? (
        <p className={`ouvrir-resultat text-sm ${said.ok ? "text-foreground" : "text-destructive"}`}>
          {said.ok ? "Ouvert — " : "Refusé — "}
          {said.text}
        </p>
      ) : null}
    </form>
  );
}
