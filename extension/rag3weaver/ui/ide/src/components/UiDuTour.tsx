import { createContext, useContext } from "react";
import type { Rag, UiComposant } from "@/lib/host";
import { avancementText, useAvancement, useFinDIndex } from "@/lib/progres";

// **Une interface qui glisse dans le chat** (Lucie, 8 novembre 2026 à 16 h 30 : « que ça
// glisse une UI dans le chat ») : un résultat d'outil porte, hors du texte que le modèle
// lit, des composants `ui` ({kind, sujets, …}) que la page rend sous l'appel. `kind` est
// un nom de composant, d'un registre ; chacun lit le flux déjà ouvert sur les sujets qu'il
// déclare, sans jamais relire l'hôte (« par SSE, pas par polling »). `index_progress` est
// le premier : l'avancement de l'index d'un dépôt.
const REGISTRE: Record<string, (p: { u: UiComposant }) => React.ReactNode> = {
  index_progress: CarteAvancement,
};

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
