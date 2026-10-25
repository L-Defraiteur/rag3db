import { useEffect, useRef, useState } from "react";
import { Check, X, Bot } from "lucide-react";
import { Button } from "@/components/ui/button";
import { lineDiff } from "@/lib/diff";
import { GraphDiffSection, type GraphDiff } from "@/lib/graph";
import { shortName, type HostState } from "@/lib/host";

// **La relecture d'une tâche** (page 08, vue 5) : la branche et son départ, les
// témoins de la branche, fusionner ou rejeter, les graphes changés (en texte :
// nœuds ajoutés / changés / retirés, arêtes — le dessin vient avec la vue 7), la
// différence ligne à ligne de chaque fichier, et l'équipier de la tâche (ses
// plans, sa boîte, de quoi lui écrire). Repères gardés : `#temoins`, `#fusionner`,
// `#rejeter`, `#pourquoi`, `#message`, `#tache-note`, `section.fichier-de-tache
// [data-genre][data-path] .diff .plus|.moins`, `section.graphe-propose[data-cible]`
// (ses nœuds en `li.ajoute|change|retire[data-nom]`).
type File = { genre: string; path: string; avant: string | null; apres: string | null };
type Review = { tache: { nom: string; id: string; branche: string; depart: string }; fichiers: File[]; graphes: GraphDiff[]; temoins: { chargement: boolean; refus?: string }; base_a_bouge: boolean };
type Plan = { number: number; request: string; proposal: string; state: string; reason?: string; items: { id: string; text: string; status: string; alone: boolean; question?: string; reason?: string }[]; steps?: string[] };
type Teammate = { mailbox: Record<string, unknown>[]; plans: { plans: Plan[] }; closed: Plan[]; running: boolean };

const STATUS: Record<string, string> = { todo: "à faire", doing: "en cours", done: "fait", blocked: "bloqué", dropped: "abandonné" };

function Diff({ before, after }: { before: string; after: string }) {
  const d = lineDiff(before, after);
  if (typeof d === "string") return <div className="diff font-mono text-xs"><div>{d}</div></div>;
  return (
    <div className="diff font-mono text-xs leading-5 overflow-x-auto">
      {d.map((l, k) => (
        <div key={k} className={`${l.kind} ${l.kind === "plus" ? "bg-green-500/15" : "bg-red-500/15"} whitespace-pre px-2`}>
          {l.text}
        </div>
      ))}
    </div>
  );
}

function PlanCard({ p, label }: { p: Plan; label: string }) {
  return (
    <div className="plan rounded-lg border px-3 py-2 text-sm" data-number={p.number} data-state={p.state}>
      <h3 className="font-semibold">Plan n° {p.number} — {label}{p.reason ? ` (${p.reason})` : ""}</h3>
      <p><b>Demande :</b> {p.request}</p>
      <p><b>Proposition :</b> {p.proposal}</p>
      <ul className="list-disc pl-5">
        {p.items.map((i) => (
          <li key={i.id} data-status={i.status}>
            <code>{i.id}</code> {i.text} — {STATUS[i.status] ?? i.status}{i.alone ? "" : " · à valider"}
            {i.question ? <span className="note text-muted-foreground"> — question : {i.question}</span> : null}
            {i.reason ? <span className="note text-muted-foreground"> — {i.reason}</span> : null}
          </li>
        ))}
      </ul>
      {p.steps?.length ? <p className="note text-xs text-muted-foreground">{p.steps.length} pas</p> : null}
    </div>
  );
}

function TeammateSection({ backend, task }: { backend: string; task: string }) {
  const [t, setT] = useState<Teammate | null>(null);
  const [err, setErr] = useState<string | null>(null);
  const kind = useRef<HTMLSelectElement>(null);
  const item = useRef<HTMLInputElement>(null);
  const text = useRef<HTMLInputElement>(null);
  const [note, setNote] = useState("");
  const load = () =>
    fetch(`/ide/teammate?${new URLSearchParams({ backend, task })}`)
      .then((r) => r.json())
      .then((r) => (r.ok ? setT(r.result) : setErr(r.error)));
  useEffect(() => {
    load();
  }, [backend, task]);
  const send = async () => {
    setNote("envoi…");
    const r = await (await fetch("/ide/teammate/mailbox", { method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify({ backend, task, kind: kind.current?.value ?? "request", item: item.current?.value || undefined, text: text.current?.value ?? "", from: "ide" }) })).json();
    if (!r.ok) {
      setNote(r.error);
      return;
    }
    setNote(r.result.started ? "envoyé ; l'équipier part" : "envoyé");
    if (text.current) text.current.value = "";
    await load();
  };
  if (err) return <section className="equipier mt-6"><h2 className="text-base font-semibold">Équipier</h2><p className="refus text-destructive text-sm">{err}</p></section>;
  if (!t) return null;
  const active = t.plans.plans.filter((p) => p.state === "active");
  const paused = t.plans.plans.filter((p) => p.state === "paused");
  return (
    <section className="equipier mt-6 grid gap-3" data-running={String(t.running)}>
      <h2 className="text-base font-semibold inline-flex items-center gap-2"><Bot className="size-4" /> Équipier{t.running ? " — en train de travailler" : ""}</h2>
      {active.length || paused.length || t.closed.length ? null : <p className="note text-sm text-muted-foreground">Pas de plan : une petite demande se fait sans.</p>}
      {active.map((p) => <PlanCard key={p.number} p={p} label="actif" />)}
      {paused.map((p) => <PlanCard key={p.number} p={p} label="en pause" />)}
      {t.closed.length ? (
        <details>
          <summary className="cursor-pointer text-sm">{t.closed.length} plan(s) clos</summary>
          <div className="grid gap-2 mt-2">{t.closed.map((p) => <PlanCard key={p.number} p={p} label={p.state === "accomplished" ? "accompli" : "obsolète"} />)}</div>
        </details>
      ) : null}
      <h3 className="text-sm font-semibold">Boîte aux lettres</h3>
      {t.mailbox.length ? (
        <ul className="boite grid gap-1 text-sm">
          {t.mailbox.map((l, k) => (
            <li key={k} data-kind={String(l.kind)} data-from={String(l.from)}>
              <span className="meta text-xs text-muted-foreground">{new Date(Number(l.at)).toLocaleTimeString()} · {String(l.from)} · {String(l.kind)}{l.item ? ` [${String(l.item)}]` : ""} </span>
              {String(l.text ?? "")}
            </li>
          ))}
        </ul>
      ) : (
        <p className="note text-sm text-muted-foreground">Rien encore.</p>
      )}
      <div className="actions flex flex-wrap items-center gap-2">
        <select id="equipier-kind" aria-label="genre du message" ref={kind} defaultValue="request" className="rounded-md border bg-input/30 px-2 py-1.5 text-sm">
          <option value="request">demande</option>
          <option value="instruction">consigne</option>
          <option value="answer">réponse à un bloquant</option>
          <option value="stop">arrêt</option>
        </select>
        <input id="equipier-item" ref={item} placeholder="élément (pour une réponse)" aria-label="élément" className="rounded-md border bg-input/30 px-2 py-1.5 text-sm" />
        <input id="equipier-text" ref={text} placeholder="écrire à l'équipier" aria-label="message à l'équipier" className="flex-1 min-w-48 rounded-md border bg-input/30 px-2 py-1.5 text-sm" />
        <Button id="equipier-envoyer" size="sm" onClick={send}>Écrire</Button>
        <span className="note text-xs text-muted-foreground" id="equipier-note">{note}</span>
      </div>
    </section>
  );
}

export function TaskReview({ id, name, state }: { id: string; name: string; state: HostState }) {
  const backend = state.backends.find((b) => b.id === id);
  const [r, setR] = useState<Review | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [note, setNote] = useState("");
  const why = useRef<HTMLInputElement>(null);
  const message = useRef<HTMLInputElement>(null);
  useEffect(() => {
    fetch(`/ide/taches?${new URLSearchParams({ backend: id, nom: name })}`)
      .then((x) => x.json())
      .then((x) => (x.ok ? setR(x.result) : setError(x.error)))
      .catch((e: Error) => setError(e.message));
  }, [id, name]);
  const act = async (route: "fusionner" | "rejeter", body: Record<string, string>) => {
    setNote(route === "fusionner" ? "fusion…" : "rejet…");
    const x = await (await fetch(`/ide/taches/${route}`, { method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify({ backend: id, nom: name, ...body }) })).json();
    if (!x.ok) {
      setNote(x.error);
      return;
    }
    // La tâche est fermée : la liste la montre parmi les fermées.
    location.assign(`/ide/code/${id}/taches/`);
  };
  if (!backend) return <p className="text-destructive">backend inconnu : {id}</p>;
  return (
    <>
      <p id="fil" className="text-xs text-muted-foreground mb-3">
        <a href="/ide/" className="hover:underline">backends</a> · <a href={`/ide/code/${id}/taches/`} className="hover:underline">{shortName(backend.manifest)}</a> · tâche {name}
      </p>
      {error ? <p className="text-destructive" role="alert">{error}</p> : null}
      {r ? (
        <div className="proposition grid gap-3">
          <h1 className="text-xl font-semibold">Tâche {r.tache.nom}</h1>
          <p className="note text-sm text-muted-foreground">
            {r.tache.branche}, partie de {r.tache.depart.slice(0, 9)}{r.base_a_bouge ? " — la base du dépôt a bougé depuis : la fusion rebasera" : ""}. <a href={`/ide/code/${r.tache.id}/arbre/`} className="underline">ses vues</a> · <a href={`/ide/code/${r.tache.id}/chat`} className="underline">son chat</a>
          </p>
          <p id="temoins" className="text-sm">{r.temoins.chargement ? "Témoins : la branche se charge entière." : "Témoins : la branche ne se charge pas."}</p>
          {r.temoins.refus ? <pre className="refus text-destructive text-xs whitespace-pre-wrap">{r.temoins.refus}</pre> : null}
          <div className="actions flex flex-wrap items-center gap-2">
            <input id="message" ref={message} placeholder={`message (la tâche ${name})`} aria-label="message de la fusion" className="rounded-md border bg-input/30 px-2 py-1.5 text-sm" />
            <Button id="fusionner" size="sm" onClick={() => act("fusionner", { message: message.current?.value ?? "" })}><Check className="size-3.5" /> Fusionner</Button>
            <input id="pourquoi" ref={why} placeholder="pourquoi rejeter" aria-label="pourquoi rejeter" className="rounded-md border bg-input/30 px-2 py-1.5 text-sm" />
            <Button id="rejeter" size="sm" variant="outline" onClick={() => act("rejeter", { why: why.current?.value || "rejetée dans l'IDE" })}><X className="size-3.5" /> Rejeter</Button>
            <span className="note text-xs text-muted-foreground" id="tache-note">{note}</span>
          </div>
          {r.fichiers.length ? null : <p className="note text-sm text-muted-foreground">La branche ne change encore rien.</p>}
          {r.graphes.map((g, k) => <GraphDiffSection key={k} g={g} />)}
          {r.fichiers.map((f) => (
            <section key={f.path} className="fichier-de-tache rounded-lg border overflow-hidden" data-genre={f.genre} data-path={f.path}>
              <h2 className="text-sm font-semibold px-3 py-2 border-b bg-muted/40">{(f.genre === "code" ? "code · " : f.genre === "depot" ? "dépôt · " : "") + f.path + (f.avant == null ? " (neuf)" : f.apres == null ? " (retiré)" : "")}</h2>
              <Diff before={f.avant ?? ""} after={f.apres ?? ""} />
            </section>
          ))}
          <TeammateSection backend={id} task={name} />
        </div>
      ) : error ? null : (
        <p className="note text-muted-foreground">Chargement…</p>
      )}
    </>
  );
}
