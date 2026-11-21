import { StrictMode, useEffect, useState } from "react";
import { createRoot } from "react-dom/client";
import { Moon, Sun, Radio } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { dispatchFlux, etatPartage, fetchChat, fetchState, jeSuisLaFenetre, onEtat, onFlux, openFlux, roleDeLaFenetre, route, type ChatTurn, type FluxStatus, type HostState, type UiComposant } from "@/lib/host";
import { Chat } from "@/views/Chat";
import { File } from "@/views/File";
import { Search } from "@/views/Search";
import { Recherches } from "@/views/Recherches";
import { Tasks } from "@/views/Tasks";
import { TaskReview } from "@/views/TaskReview";
import { Bilans } from "@/views/Bilans";
import { Tree } from "@/views/Tree";
import { Symbol } from "@/views/Symbol";
import { GraphView } from "@/views/Graph";
import { Dag } from "@/views/Dag";
import { Reactions } from "@/views/Reactions";
import { Propositions } from "@/views/Propositions";
import { applyTheme, effectiveTheme, storedTheme, themesLivres, type Theme } from "@/lib/theme";
import { Home } from "@/views/Home";
import { Poste } from "@/views/Poste";

// **Le cadre de l'IDE** (page 08, vue 1) : l'en-tête, le flux ouvert une fois,
// l'état de l'hôte relu à chaque ouverture du flux, et la vue que l'URL nomme.
// Les vues non portées sont servies par l'hôte depuis l'ancienne page : la
// route de l'hôte choisit, la page ne les connaît pas.
/// **The approvals awaited, in a window that is not the chat** (the IDE in several windows, slice 5; ED's review: from
/// what the host already says, without a shared key). At its load, the window reads the chat's state once, and the
/// decision of each approval card it finds there; then it follows the stream only: a `ToolCallFinished` of its backend
/// that carries an approval card adds it, an `AccordDecide` takes it away. No request while idle.
function AccordsEnAttente({ id, manifest }: { id: string; manifest: string | undefined }) {
  const [enAttente, setEnAttente] = useState<Set<string>>(new Set());
  // The load done (the chat's state and each card's decision read): a mark for the witnesses, an order not a delay.
  const [charge, setCharge] = useState(false);
  useEffect(() => {
    let vivant = true;
    setCharge(false);
    const decides = new Set<string>();
    const ajouter = (ids: string[]) =>
      setEnAttente((cur) => {
        const n = new Set(cur);
        for (const i of ids) if (!decides.has(i)) n.add(i);
        return n.size === cur.size ? cur : n;
      });
    const retirer = (i: string) => {
      decides.add(i);
      setEnAttente((cur) => {
        if (!cur.has(i)) return cur;
        const n = new Set(cur);
        n.delete(i);
        return n;
      });
    };
    const accords = (ui: UiComposant[] | undefined) => (ui ?? []).filter((u) => u.kind === "accord" && u.id).map((u) => String(u.id));
    fetchChat(id)
      .then((s) =>
        Promise.all(
          s.turns.flatMap((t: ChatTurn) => accords(t.ui)).map((i) =>
            fetch(`/ide/accords?id=${encodeURIComponent(i)}`)
              .then((r) => r.json())
              // Awaited only when the host knows the card and it has no decision (ED's review): requests live in
              // the host's memory, and a card it no longer knows (a restart) can never be decided.
              .then((v: { ok?: boolean; result?: { decision?: string | null } }) => ({ i, etat: v.ok && v.result ? (v.result.decision ? "decide" : "attente") : "inconnue" }))
              .catch(() => ({ i, etat: "inconnue" })),
          ),
        ),
      )
      .then((rs) => {
        if (!vivant) return;
        for (const r of rs) {
          if (r.etat === "decide") retirer(r.i);
          else if (r.etat === "attente") ajouter([r.i]);
        }
      })
      .catch(() => {})
      .finally(() => vivant && setCharge(true));
    const off = onFlux((d) => {
      const e = d.evenement ?? {};
      if (d.sujet === "accords" && e.kind === "AccordDecide" && e.id) retirer(String(e.id));
      else if (e.kind === "ToolCallFinished" && d.backend === manifest && Array.isArray(e.ui)) ajouter(accords(e.ui as UiComposant[]));
    });
    return () => {
      vivant = false;
      off();
    };
  }, [id, manifest]);
  const n = enAttente.size;
  return (
    <span id="accords-en-attente" data-compte={String(n)} data-charge={charge ? "1" : "0"} className={n ? "text-xs text-primary" : "hidden"}>
      {n === 1 ? "1 accord attend dans le chat" : `${n} accords attendent dans le chat`}
    </span>
  );
}

/// **The index of the window's repository is ready** (ticket of 20 Nov. 2026, « l'avis index prêt dans la fenêtre
/// détachée du dépôt »): the host's watch says `IndexReady` once on the stream, subject `index`; a detached window of
/// that backend says it in its header. From the stream only: no request, at rest or otherwise.
function IndexPretFenetre({ manifest }: { manifest: string | undefined }) {
  const [avis, setAvis] = useState<{ texte: string; recus: number }>({ texte: "", recus: 0 });
  useEffect(
    () =>
      onFlux((d) => {
        const e = d.evenement ?? {};
        if (d.sujet === "index" && e.kind === "IndexReady" && manifest && d.backend === manifest)
          setAvis((a) => ({ texte: String(e.text ?? "l'index est prêt"), recus: a.recus + 1 }));
      }),
    [manifest],
  );
  return (
    <span id="index-pret-fenetre" role="status" data-recus={String(avis.recus)} className={avis.recus ? "text-xs text-primary" : "hidden"}>
      {avis.texte}
    </span>
  );
}

function App() {
  const [state, setState] = useState<HostState | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [flux, setFlux] = useState<FluxStatus>("fermé");
  const [theme, setTheme] = useState<Theme>(storedTheme());
  const [livres, setLivres] = useState<string[]>([]);
  // **A detached window** (the IDE in several windows, slice 3): its role, and a render on each internal navigation.
  const fenetre = roleDeLaFenetre();
  const [, setNavigation] = useState(0);
  useEffect(() => {
    const relire = () => setNavigation((n) => n + 1);
    window.addEventListener("popstate", relire);
    return () => window.removeEventListener("popstate", relire);
  }, []);

  // Un nom que l'hôte ne connaît pas revient au choix du système : l'état le suit.
  useEffect(() => applyTheme(theme, (applique) => applique !== theme && setTheme(applique)), [theme]);
  useEffect(() => {
    themesLivres()
      .then(setLivres)
      .catch((e: Error) => console.warn(`[thème] ${e.message}`));
  }, []);
  const reloadState = () => fetchState().then(setState).catch((e: Error) => setError(e.message));
  // Un worktree attaché par l'atelier (`RepoAttached`), puis la fin de son premier
  // index (`RepoIndexed`) : l'hôte pousse son état, la barre le montre sous « Dépôts »,
  // puis son rag indexé, sans recharger la page ni relire l'hôte.
  useEffect(() => onEtat<HostState>("hote", "*", (s) => setState(s)), []);
  // **A detached window of the repository follows the selection** (slice 2): the one of its backend that had the
  // focus last opens what the chat cites, and stays a window of the repository. A selection older than the window
  // (received whole when it opens) is not followed.
  // Slice 4: the window of the graph follows the tools of the turn, under its own key (`graphe:<backend>`).
  useEffect(() => {
    const id = route().id;
    const role = roleDeLaFenetre();
    const cle = role === "depot" ? "selection" : role === "graphe" ? "graphe" : null;
    if (!role || !cle || !id) return;
    const ouverte = Date.now();
    return etatPartage.suivre(`${cle}:${id}`, (v) => {
      const s = v as { href?: string; at?: number } | undefined;
      if (!s?.href || (s.at ?? 0) < ouverte || !jeSuisLaFenetre(role, id)) return;
      // By `URL` (ED's review): a cited address with its own query keeps it, and the window its role; the hash follows.
      const u = new URL(s.href, location.origin);
      u.searchParams.set("fenetre", role);
      // Slice 3: an internal navigation — the page is not reloaded, it keeps its link to the worker, and the view
      // renders again from the new address.
      history.pushState(null, "", u.toString());
      setNavigation((n) => n + 1);
    });
  }, []);
  useEffect(() => {
    reloadState();
    return openFlux(dispatchFlux, (s: FluxStatus) => {
      setFlux(s);
      // La coupure se dit aussi aux vues (l'avancement des index : « flux coupé »).
      if (s !== "ouvert") dispatchFlux({ sujet: "flux", backend: "*", evenement: { kind: "coupe" } });
      if (s === "ouvert") {
        reloadState();
        // Les vues relisent leur état à chaque ouverture du flux (le chat : un
        // tour parti avant, à rattacher).
        dispatchFlux({ sujet: "flux", backend: "*", evenement: { kind: "ouvert" } });
      }
    });
  }, []);

  const dark = effectiveTheme(theme) !== "light";
  const fichier = livres.includes(theme) ? theme : "";
  const fullWindow = route().view === "chat";
  return (
    // Le chat prend toute la fenêtre : ses barres aux bords, un seul défilement
    // (sa liste) ; les autres vues gardent leur largeur de lecture et la page
    // défile (ticket « le chat a une largeur fixe sur un écran large »).
    <div className={`${fullWindow ? "h-dvh overflow-hidden" : "min-h-dvh"} flex flex-col bg-background text-foreground`}>
      <header className="flex items-center gap-3 px-4 py-2 border-b" id="entete" data-fenetre={fenetre ?? undefined}>
        {/* A detached window says its role and the stream only (slice 3). */}
        {fenetre ? (
          <>
            <span className="text-xs text-muted-foreground">{`fenêtre : ${fenetre === "depot" ? "dépôt" : fenetre}`}</span>
            {route().id ? <AccordsEnAttente id={route().id!} manifest={state?.backends.find((b) => b.id === route().id)?.manifest} /> : null}
            {route().id ? <IndexPretFenetre manifest={state?.backends.find((b) => b.id === route().id)?.manifest} /> : null}
          </>
        ) : (
          <>
            <a href="/ide/" className="font-semibold tracking-tight">
              rag3weaver
            </a>
            <span className="text-xs text-muted-foreground">IDE</span>
          </>
        )}
        <span className="ml-auto flex items-center gap-1 text-xs text-muted-foreground">
          <Radio className="size-3" aria-hidden />
          {/* Le texte exact « flux : ouvert » : les témoins le lisent à l'égal. */}
          <span id="lien" data-flux={flux}>{`flux : ${flux}`}</span>
        </span>
        {fenetre ? null : (
          <>
            <Button
              variant="ghost"
              size="icon"
              id="theme"
              aria-label={dark ? "passer en clair" : "passer en sombre"}
              title={dark ? "passer en clair" : "passer en sombre"}
              onClick={() => setTheme(dark ? "light" : "dark")}
            >
              {dark ? <Sun className="size-4" /> : <Moon className="size-4" />}
            </Button>
            {/* Les thèmes du terminal (templates/themes/<nom>.json), ceux que l'hôte sert. */}
            <select
              id="theme-fichier"
              aria-label="thème"
              title="thème"
              className="h-8 rounded-md border bg-background px-2 text-xs text-foreground"
              value={fichier}
              onChange={(e) => setTheme(e.target.value || "system")}
            >
              <option value="">thème…</option>
              {livres.map((nom) => (
                <option key={nom} value={nom}>
                  {nom}
                </option>
              ))}
            </select>
          </>
        )}
      </header>
      <main id="vue" className={fullWindow ? "flex-1 min-h-0 w-full flex flex-col" : "flex-1 px-4 py-6 max-w-5xl w-full mx-auto"}>
        {error ? (
          <p className="text-destructive" role="alert">
            {error}
          </p>
        ) : state ? (
          <View state={state} reload={reloadState} />
        ) : (
          <p className="text-muted-foreground">
            <Badge variant="outline">…</Badge> l'état de l'hôte se lit
          </p>
        )}
      </main>
    </div>
  );
}

// La vue que l'URL nomme ; une vue non portée n'arrive pas ici : l'hôte la sert
// depuis l'ancienne page.
function View({ state, reload }: { state: HostState; reload: () => Promise<unknown> }) {
  const r = route();
  if (r.view === "taches" && r.id) return r.rest ? <TaskReview id={r.id} name={r.rest} state={state} /> : <Tasks id={r.id} state={state} onChanged={reload} />;
  if (r.view === "chat" && r.id) return <Chat id={r.id} state={state} />;
  if (r.view === "fichier" && r.id) return <File id={r.id} path={r.rest ?? ""} state={state} />;
  if (r.view === "recherche" && r.id) return <Search id={r.id} state={state} />;
  if (r.view === "recherches" && r.id) return <Recherches id={r.id} state={state} />;
  if (r.view === "bilans" && r.id) return <Bilans id={r.id} state={state} />;
  if (r.view === "arbre" && r.id) return <Tree id={r.id} dir={r.rest ?? ""} state={state} />;
  if (r.view === "symbole" && r.id) return <Symbol id={r.id} name={r.rest ?? ""} state={state} />;
  if (r.view === "graphe" && r.id) return <GraphView id={r.id} name={r.rest ?? ""} state={state} />;
  if (r.view === "dag" && r.id) return <Dag id={r.id} rest={r.rest ?? ""} state={state} />;
  if (r.view === "reactions" && r.id) return <Reactions id={r.id} state={state} />;
  if (r.view === "propositions" && r.id) return <Propositions id={r.id} rest={r.rest ?? ""} state={state} />;
  if (r.view === "poste") return <Poste />;
  return <Home state={state} />;
}

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <App />
  </StrictMode>,
);
