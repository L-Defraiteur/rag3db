import { useEffect, useRef, useState } from "react";
import { GitBranch, Plus } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { shortName, type HostState } from "@/lib/host";

// **Les tâches de l'agent** (page 08, vue 5) : la liste des tâches ouvertes (sa
// branche, sa copie de base, ses vues), celles fermées (fusionnée, rejetée,
// orpheline), et l'ouverture d'une tâche. Repères de l'ancienne page gardés :
// `#tache-nom`, `#ouvrir-tache`, `#tache-note`, `#taches tr[data-nom]`,
// `#taches-fermees li[data-nom][data-etat]`.
type Open = { nom: string; id: string; branche: string; copie: { reflink: boolean; octets: number; tenue_ms: number } };
type Closed = { nom: string; etat: string; commit?: string; pourquoi?: string };

export function Tasks({ id, state, onChanged }: { id: string; state: HostState; onChanged: () => Promise<unknown> }) {
  const backend = state.backends.find((b) => b.id === id);
  const [list, setList] = useState<{ ouvertes: Open[]; fermees: Closed[] } | null>(null);
  const [note, setNote] = useState("");
  // Les champs ne sont pas contrôlés par React : une valeur posée par le DOM
  // (un témoin, une saisie automatique) se lit là.
  const name = useRef<HTMLInputElement>(null);
  const load = async () => {
    const r = await (await fetch(`/ide/taches?${new URLSearchParams({ backend: id })}`)).json();
    if (!r.ok) throw new Error(r.error);
    setList(r.result);
  };
  useEffect(() => {
    load().catch((e: Error) => setNote(e.message));
  }, [id]);
  const open = async () => {
    setNote("copie de la base, worktree…");
    const r = await (await fetch("/ide/taches", { method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify({ backend: id, nom: name.current?.value ?? "" }) })).json();
    if (!r.ok) {
      setNote(r.error);
      return;
    }
    setNote("");
    if (name.current) name.current.value = "";
    await onChanged();
    await load();
  };
  if (!backend) return <p className="text-destructive">backend inconnu : {id}</p>;
  return (
    <>
      <p id="fil" className="text-xs text-muted-foreground mb-3">
        <a href="/ide/" className="hover:underline">backends</a> · {shortName(backend.manifest)} · tâches
      </p>
      <h1 className="text-xl font-semibold mb-1">Tâches de l'agent</h1>
      <p className="note text-sm text-muted-foreground mb-4">Une tâche : une branche à elle, un worktree, une copie de la base. L'agent y déclare ; on relit, puis on fusionne ou on rejette.</p>
      <div className="barre flex flex-wrap items-center gap-2 mb-4">
        <input id="tache-nom" ref={name} placeholder="nom de la tâche" aria-label="nom de la tâche" className="rounded-md border bg-input/30 px-3 py-1.5 text-sm" />
        <Button id="ouvrir-tache" size="sm" onClick={open}>
          <Plus className="size-3.5" /> Ouvrir une tâche
        </Button>
        <span className="note text-xs text-muted-foreground" id="tache-note">{note}</span>
      </div>
      {list ? (
        <>
          {list.ouvertes.length ? (
            <table className="runs w-full text-sm" id="taches">
              <thead className="text-left text-xs text-muted-foreground">
                <tr>{["tâche", "branche", "base", "backend"].map((t) => <th key={t} className="py-1 pr-3 font-medium">{t}</th>)}</tr>
              </thead>
              <tbody>
                {list.ouvertes.map((t) => (
                  <tr key={t.nom} data-nom={t.nom} className="border-t">
                    <td className="py-1.5 pr-3"><a href={`/ide/code/${id}/taches/${encodeURIComponent(t.nom)}`} className="hover:underline inline-flex items-center gap-1"><GitBranch className="size-3.5" /> {t.nom}</a></td>
                    <td className="py-1.5 pr-3 font-mono text-xs">{t.branche}</td>
                    <td className="py-1.5 pr-3 text-xs text-muted-foreground">{t.copie.reflink ? "reflink" : "copie pleine"}, {Math.round(t.copie.octets / 1024)} Kio, porte {t.copie.tenue_ms} ms</td>
                    <td className="py-1.5 text-xs"><a href={`/ide/code/${t.id}/arbre/`} className="hover:underline">ses vues</a> · <a href={`/ide/code/${t.id}/propositions/`} className="hover:underline">ses propositions</a> · <a href={`/ide/code/${t.id}/chat`} className="hover:underline">chat</a></td>
                  </tr>
                ))}
              </tbody>
            </table>
          ) : (
            <p className="note text-sm text-muted-foreground">Aucune tâche ouverte.</p>
          )}
          {list.fermees.length ? (
            <>
              <h2 className="text-base font-semibold mt-6 mb-2">Fermées</h2>
              <ul className="liste grid gap-1 text-sm" id="taches-fermees">
                {list.fermees.map((t, k) => (
                  <li key={`${t.nom}-${k}`} data-nom={t.nom} data-etat={t.etat} className="flex items-center gap-2">
                    {t.nom}
                    <Badge variant="outline" className="meta font-normal">
                      {t.etat === "fusionnee" ? `fusionnée (${String(t.commit).slice(0, 9)})` : t.etat === "orpheline" ? `orpheline : ${t.pourquoi}` : `rejetée : ${t.pourquoi}`}
                    </Badge>
                  </li>
                ))}
              </ul>
            </>
          ) : null}
        </>
      ) : (
        <p className="note text-muted-foreground">Chargement…</p>
      )}
    </>
  );
}
