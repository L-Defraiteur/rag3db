import { useEffect, useState } from "react";
import { Bot, ChevronDown, ChevronRight, CircleAlert, MessageSquare } from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { displayName, fetchPoste, type HostState, type Poste as PosteState } from "@/lib/host";

// **The welcome is the tree of the agents** (Lucie, 21 November 2026: « la liste des agents racines, quelles tâches a
// chacun, plans en cours, à coup de dépliage de hiérarchie » ; « ce qui attend une intervention » first). Read from the
// host without a new route: the live sessions of `GET /poste` (by their Claude name), the open tasks of each repository
// (`/ide/taches`, a sub-agent carries `parent_session`), the conversations of `/ide/etat`, and, unfolded, the « doing »
// item of a sub-agent's plan (`/ide/teammate`). `GET /ide/agents` (archi) will replace the gluing. Markers of the
// witness: `#a-intervenir`, `#agents-arbre`, `.agent[data-agent]`, `.deplier`, `.sous-agent`.

type Tache = { nom: string; parent_session?: string | null };
type SousAgent = { backend: string; depot: string; tache: Tache };
type Racine = { cle: string; nom: string; sorte: "claude" | "conversation" | "ide"; actif: boolean; detail?: string; lien?: string; enfants: SousAgent[] };

export function Agents({ state }: { state: HostState }) {
  const [poste, setPoste] = useState<PosteState | null>(null);
  const [taches, setTaches] = useState<SousAgent[]>([]);
  const principaux = (state.backends ?? []).filter((b) => !b.tache && b.kind !== "memory");
  const cle = principaux.map((b) => b.id).join(",");
  useEffect(() => {
    let vivant = true;
    const lire = async () => {
      const p = await fetchPoste().catch(() => null);
      const listes = await Promise.all(
        principaux.map(async (b) => {
          try {
            const r = await (await fetch(`/ide/taches?${new URLSearchParams({ backend: b.id })}`)).json();
            return ((r.ok ? r.result?.ouvertes : null) ?? []).map((t: Tache) => ({ backend: b.id, depot: displayName(b), tache: t }));
          } catch {
            return [];
          }
        }),
      );
      if (vivant) {
        setPoste(p);
        setTaches(listes.flat());
      }
    };
    lire();
    const t = setInterval(lire, 5000);
    return () => {
      vivant = false;
      clearInterval(t);
    };
  }, [cle]); // eslint-disable-line react-hooks/exhaustive-deps

  // The roots: the live Claude sessions, the names sub-agents carry (inactive without a live session), the
  // conversations of the repositories, and the tasks launched from the IDE.
  const racines: Racine[] = [];
  const vivants = new Map<string, NonNullable<PosteState["sessions"][number]>>();
  for (const s of poste?.sessions ?? []) if (s.claude?.name) vivants.set(s.claude.name, s);
  const noms = new Set<string>([...vivants.keys(), ...taches.map((t) => t.tache.parent_session).filter((n): n is string => !!n)]);
  for (const nom of [...noms].sort((a, b) => Number(vivants.has(b)) - Number(vivants.has(a)) || a.localeCompare(b))) {
    const s = vivants.get(nom);
    racines.push({
      cle: `claude:${nom}`,
      nom,
      sorte: "claude",
      actif: !!s,
      detail: s ? `${s.client}${s.pid ? ` · pid ${s.pid}` : ""}` : "sans session ouverte",
      enfants: taches.filter((t) => t.tache.parent_session === nom),
    });
  }
  for (const s of poste?.sessions ?? []) {
    if (s.claude?.name || s.client === "ide") continue;
    racines.push({ cle: `sans-nom:${s.id}`, nom: "sans nom", sorte: "claude", actif: true, detail: `${s.client} : serveur MCP d'avant le 21 novembre, à reconnecter (/mcp)`, enfants: [] });
  }
  for (const c of state.chat_sessions ?? []) {
    racines.push({ cle: `conversation:${c.id}`, nom: c.name || c.id, sorte: "conversation", actif: c.state === "running" || c.state === "waiting", lien: `/ide/code/${c.id}/chat`, enfants: [] });
  }
  const sansParent = taches.filter((t) => !t.tache.parent_session);
  if (sansParent.length) racines.push({ cle: "ide", nom: "lancés depuis l'IDE", sorte: "ide", actif: true, enfants: sansParent });

  // What waits for an intervention: a task or a conversation that waits for an answer (a question, an accord).
  const attentes = [
    ...(state.tasks ?? []).filter((t) => t.awaits_answer).map((t) => ({ cle: `t:${t.id}`, texte: `le sous-agent ${t.name} attend une réponse`, lien: `/ide/code/${t.session}/taches/${encodeURIComponent(t.name)}` })),
    ...(state.chat_sessions ?? []).filter((c) => c.awaits_answer).map((c) => ({ cle: `c:${c.id}`, texte: `la conversation ${c.name || c.id} attend une réponse`, lien: `/ide/code/${c.id}/chat` })),
  ];

  return (
    <>
      <section id="a-intervenir" className="mt-4">
        <h2 className="text-base font-semibold mb-2 flex items-center gap-2">
          <CircleAlert className="size-4" /> À intervenir
        </h2>
        {attentes.length ? (
          <ul className="text-sm space-y-1">
            {attentes.map((a) => (
              <li key={a.cle}>
                <a href={a.lien} className="hover:underline">{a.texte}</a>
              </li>
            ))}
          </ul>
        ) : (
          <p className="text-sm text-muted-foreground">Rien n'attend une intervention.</p>
        )}
      </section>
      <section id="agents-arbre" className="mt-6">
        <h2 className="text-base font-semibold mb-2 flex items-center gap-2">
          <Bot className="size-4" /> Agents
        </h2>
        {racines.length ? (
          <ul className="text-sm space-y-1">
            {racines.map((r) => (
              <NoeudRacine key={r.cle} racine={r} />
            ))}
          </ul>
        ) : (
          <p className="text-sm text-muted-foreground">Aucun agent pour l'instant.</p>
        )}
      </section>
    </>
  );
}

function NoeudRacine({ racine }: { racine: Racine }) {
  const [ouvert, setOuvert] = useState(false);
  return (
    <li className="agent" data-agent={racine.nom}>
      <div className="flex flex-wrap items-center gap-2">
        <button type="button" className="deplier inline-flex" aria-label={ouvert ? "replier" : "déplier"} onClick={() => setOuvert(!ouvert)} disabled={!racine.enfants.length}>
          {ouvert ? <ChevronDown className="size-4" /> : <ChevronRight className={`size-4 ${racine.enfants.length ? "" : "opacity-30"}`} />}
        </button>
        {racine.sorte === "conversation" ? <MessageSquare className="size-4" /> : <Bot className="size-4" />}
        {racine.lien ? (
          <a href={racine.lien} className="font-medium hover:underline">{racine.nom}</a>
        ) : (
          <span className="font-medium">{racine.nom}</span>
        )}
        <Badge variant={racine.actif ? "default" : "secondary"}>{racine.actif ? "actif" : "inactif"}</Badge>
        {racine.enfants.length ? <span className="text-xs text-muted-foreground">{racine.enfants.length} sous-agent(s)</span> : null}
        {racine.detail ? <span className="text-xs text-muted-foreground">{racine.detail}</span> : null}
      </div>
      {ouvert ? (
        <ul className="ml-6 mt-1 space-y-1">
          {racine.enfants.map((e) => (
            <NoeudSousAgent key={`${e.backend}/${e.tache.nom}`} sous={e} />
          ))}
        </ul>
      ) : null}
    </li>
  );
}

/// The « doing » item of a plan, wherever the plan keeps its items.
function enCours(v: unknown): string | null {
  if (!v || typeof v !== "object") return null;
  if (Array.isArray(v)) {
    for (const x of v) {
      const t = enCours(x);
      if (t) return t;
    }
    return null;
  }
  const o = v as Record<string, unknown>;
  if (o.status === "doing") return String(o.text ?? o.title ?? o.id ?? "en cours");
  for (const x of Object.values(o)) {
    const t = enCours(x);
    if (t) return t;
  }
  return null;
}

function NoeudSousAgent({ sous }: { sous: SousAgent }) {
  const [doing, setDoing] = useState<string | null>(null);
  const [tourne, setTourne] = useState(false);
  useEffect(() => {
    fetch(`/ide/teammate?${new URLSearchParams({ backend: sous.backend, task: sous.tache.nom })}`)
      .then((r) => r.json())
      .then((r) => {
        if (!r.ok) return;
        setDoing(enCours(r.result?.plans));
        setTourne(!!r.result?.running);
      })
      .catch(() => {});
  }, [sous.backend, sous.tache.nom]);
  return (
    <li className="sous-agent flex flex-wrap items-center gap-2">
      <a href={`/ide/code/${sous.backend}/taches/${encodeURIComponent(sous.tache.nom)}`} className="hover:underline">{sous.tache.nom}</a>
      <span className="text-xs text-muted-foreground">{sous.depot}</span>
      {tourne ? <Badge>travaille</Badge> : null}
      {doing ? <span className="text-xs">en cours : {doing}</span> : null}
    </li>
  );
}
