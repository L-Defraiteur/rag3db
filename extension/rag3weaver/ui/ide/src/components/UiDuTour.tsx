import { createContext, useContext, useEffect, useState } from "react";
import { onFlux, type Rag, type UiComposant } from "@/lib/host";
import { avancementText, useAvancement, useFinDIndex } from "@/lib/progres";

// **Une interface qui glisse dans le chat** (Lucie, 8 novembre 2026 à 16 h 30 : « que ça
// glisse une UI dans le chat ») : un résultat d'outil porte, hors du texte que le modèle
// lit, des composants `ui` ({kind, sujets, …}) que la page rend sous l'appel. `kind` est
// un nom de composant, d'un registre ; chacun lit le flux déjà ouvert sur les sujets qu'il
// déclare, sans jamais relire l'hôte (« par SSE, pas par polling »). `index_progress` est
// le premier : l'avancement de l'index d'un dépôt.
const REGISTRE: Record<string, (p: { u: UiComposant }) => React.ReactNode> = {
  index_progress: CarteAvancement,
  accord: CarteAccord,
};

/// **La carte d'accord** (ask_permission ; décision de c7, 9 novembre 2026) : le chemin, le
/// mode et la raison du modèle ; « accorder » ou « refuser ». Une commande en attente (lot 7)
/// y dit sa ligne ; la décision relance le tour d'elle-même. La décision va à l'hôte, qui
/// la journalise et la publie (sujet `accords`) ; la carte la suit par le flux, et relue
/// après un rechargement, elle lit son état une fois.
function CarteAccord({ u }: { u: UiComposant }) {
  const [decision, setDecision] = useState<string | null>(null);
  const [erreur, setErreur] = useState<string | null>(null);
  useEffect(() => {
    if (!u.id) return;
    fetch(`/ide/accords?id=${encodeURIComponent(u.id)}`)
      .then((r) => r.json())
      .then((v: { ok?: boolean; result?: { decision?: string | null } }) => {
        if (v.ok && v.result?.decision) setDecision(v.result.decision);
      })
      .catch(() => {});
    return onFlux((d) => {
      const e = d.evenement ?? {};
      if (d.sujet === "accords" && e.kind === "AccordDecide" && e.id === u.id) setDecision(String(e.decision));
    });
  }, [u.id]);
  const decider = (d: "accorder" | "refuser") =>
    fetch("/ide/accords", { method: "POST", headers: { "content-type": "application/json" }, body: JSON.stringify({ id: u.id, decision: d }) })
      .then((r) => r.json())
      .then((v: { ok?: boolean; error?: string; result?: { decision?: string } }) => (v.ok ? setDecision(v.result?.decision ?? null) : setErreur(v.error ?? "refusé par l'hôte")))
      .catch((e: Error) => setErreur(e.message));
  const verbe = u.mode === "write" ? "écrire" : "lire";
  const commande = u.genre === "commande";
  return (
    <div className="carte-accord border rounded-lg px-3 py-2 text-xs" data-id={u.id ?? ""} data-decision={decision ?? ""}>
      <div className="font-medium text-foreground/90">
        {commande ? (
          <>
            l'atelier demande à lancer <span className="commande font-mono">{u.commande}</span>
          </>
        ) : (
          <>
            l'atelier demande à {verbe} <span className="font-mono">{u.chemin}</span>
          </>
        )}
      </div>
      {u.raison ? <div className="raison text-muted-foreground mt-0.5">{u.raison}</div> : null}
      {decision ? (
        <div className="decision mt-1.5 font-medium">{decision}</div>
      ) : (
        <div className="flex gap-2 mt-1.5">
          <button type="button" className="accorder border rounded px-2 py-0.5 hover:bg-accent" onClick={() => decider("accorder")}>
            accorder
          </button>
          <button type="button" className="refuser border rounded px-2 py-0.5 hover:bg-accent" onClick={() => decider("refuser")}>
            refuser
          </button>
        </div>
      )}
      {erreur ? <div className="text-destructive mt-1">{erreur}</div> : null}
    </div>
  );
}

/// Le rag de chaque dépôt (par son id) tel que l'état de l'hôte le tient déjà dans la page :
/// une carte relue après un rechargement, sans avancement sur le flux, dit par lui si
/// l'index est fini ou en cours (aucune relecture de l'hôte pour autant).
export const RagDesDepots = createContext<Map<string, Rag>>(new Map());

export function UiDuTour({ ui }: { ui?: UiComposant[] }) {
  if (!ui?.length) return null;
  return (
    <div className="ui-du-tour flex flex-col gap-2 my-2">
      {ui.map((u, k) => {
        const C = REGISTRE[u.kind];
        return C ? <C key={k} u={u} /> : null;
      })}
    </div>
  );
}

/// La carte de l'avancement d'un index : la phase, une barre, le pourcentage, le débit,
/// « figé depuis », « flux coupé » ; à la fin, ce que l'index a pris (ou son échec).
function CarteAvancement({ u }: { u: UiComposant }) {
  const { a, fige, fluxOuvert } = useAvancement(u.manifest);
  const fin = useFinDIndex(u.manifest);
  const rag = useContext(RagDesDepots).get(u.repo ?? "");
  const titre = u.titre ?? u.repo ?? "";
  // Sans rien du flux depuis l'ouverture de la page (une carte relue) : l'état connu.
  const connu = !a && !fin && rag ? (rag.state === "indexed" ? "fini" : rag.state === "indexing" ? "en-cours" : null) : null;
  const etat = a ? "en-cours" : fin ? (fin.ok ? "fini" : "echec") : (connu ?? "en-attente");
  return (
    <div className="carte-avancement border rounded-lg px-3 py-2 text-xs" data-repo={u.repo ?? ""} data-etat={etat} data-pourcent={a?.pourcent ?? ""}>
      <div className="flex justify-between gap-2">
        <span className="font-medium text-foreground/90">index {titre}</span>
        <span className="text-muted-foreground">{a ? `${a.pourcent} %` : fin ? (fin.ok ? "terminé" : "en échec") : connu === "fini" ? "prêt" : connu === "en-cours" && rag?.percent != null ? `${rag.percent} %` : "…"}</span>
      </div>
      <div className="barre h-1.5 rounded bg-muted mt-1.5 overflow-hidden" aria-hidden>
        <div className="h-full bg-primary transition-[width]" style={{ width: `${a ? a.pourcent : fin?.ok || connu === "fini" ? 100 : (rag?.percent ?? 0)}%` }} />
      </div>
      <div className="texte text-muted-foreground mt-1">
        {a
          ? avancementText(a, fige, fluxOuvert)
          : fin
            ? fin.texte
            : connu === "fini"
              ? "index prêt"
              : connu === "en-cours"
                ? `index en cours${rag?.percent != null ? ` (${rag.percent} %)` : ""} : son avancement viendra sur le flux`
                : fluxOuvert
                  ? "en attente de son avancement sur le flux"
                  : "flux coupé"}
      </div>
    </div>
  );
}
