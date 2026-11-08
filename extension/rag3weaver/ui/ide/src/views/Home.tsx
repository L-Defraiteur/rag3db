import { FolderTree, GitBranch, History, MessageSquare, Scale, Workflow, Zap, FileDiff } from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { displayName, type Backend, type HostState } from "@/lib/host";
import { SearchBox } from "@/components/SearchBox";

// **L'accueil** : les dépôts de l'hôte, chacun avec ses vues et ses tâches (Lucie, 8
// novembre 2026 : l'accueil parle en dépôts, comme la barre du chat). Une carte se nomme
// par son dépôt et sa branche (`displayName`), le chemin du manifeste n'est qu'en
// infobulle, une tâche se dit par son worktree (sa branche `agent/<nom>`) ; « backend »
// ne paraît pas à l'écran. Repères des témoins : `#vue h1` « Dépôts », `.depot[data-backend]`
// et son `.nom`, `.worktree`, un lien `/ide/code/<id>/arbre/` par dépôt.
export function Home({ state }: { state: HostState }) {
  const backends = state.backends ?? [];
  if (!backends.length) {
    return <p className="text-muted-foreground">L'hôte ne sert encore aucun dépôt : un dépôt paraît ici quand un agent ou l'atelier l'ouvre.</p>;
  }
  const principals = backends.filter((b) => !b.tache);
  const tasksOf = (b: Backend) => backends.filter((t) => t.tache && t.tache.parent === b.id);
  return (
    <>
      <h1 className="text-xl font-semibold mb-4">Dépôts</h1>
      <ul className="liste grid gap-3">
        {principals.map((b) => (
          <li key={b.id} className="depot" data-backend={b.id}>
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
