import { useEffect, useState } from "react";
import { onFlux } from "@/lib/host";

// **L'avancement des index, par le flux** (Lucie, 8 novembre 2026 à 16 h 30 et 16 h 31 :
// une barre qui avance, son pourcentage ; « par SSE, pas par polling ») : le dernier
// `IndexProgress` de chaque dépôt (par son manifeste, la clé du flux), oublié à son
// `RepoIndexed`. L'hôte ne publie qu'un avancement qui bouge : rien depuis
// `FIGE_MS` se dit « figé depuis N min ». Le flux coupé se dit aussi. Aucun composant ne
// relit l'hôte : une horloge locale fait seulement vieillir « figé depuis ».
export type Avancement = { phase: string; fait: number; total: number; pourcent: number; debit: number | null; unite: string; recu: number };

const parManifeste = new Map<string, Avancement>();
const abonnes = new Set<() => void>();
let fluxOuvert = true;
const prevenir = () => abonnes.forEach((f) => f());

onFlux((d) => {
  const e = d.evenement ?? {};
  if (d.sujet === "flux") {
    fluxOuvert = e.kind === "ouvert";
    prevenir();
  } else if (e.kind === "IndexProgress") {
    parManifeste.set(d.backend, {
      phase: String(e.phase ?? ""),
      fait: Number(e.fait ?? 0),
      total: Number(e.total ?? 0),
      pourcent: Number(e.pourcent ?? 0),
      debit: typeof e.debit === "number" ? e.debit : null,
      unite: String(e.unite ?? ""),
      recu: Date.now(),
    });
    prevenir();
  } else if (e.kind === "RepoIndexed") {
    parManifeste.delete(d.backend);
    prevenir();
  }
});

/// Le seuil du « figé » : deux minutes sans avancement ; un témoin le raccourcit.
const figeMs = () => (window as unknown as { __ideFigeMs?: number }).__ideFigeMs ?? 120_000;

/// L'avancement d'un dépôt (par son manifeste), s'il en court un, et l'état du flux.
export function useAvancement(manifeste?: string): { a: Avancement | null; fige: number | null; fluxOuvert: boolean } {
  const [, rendre] = useState(0);
  useEffect(() => {
    const f = () => rendre((n) => n + 1);
    abonnes.add(f);
    const horloge = window.setInterval(f, Math.min(15_000, Math.max(250, figeMs() / 4)));
    return () => {
      abonnes.delete(f);
      window.clearInterval(horloge);
    };
  }, []);
  const a = manifeste ? (parManifeste.get(manifeste) ?? null) : null;
  const depuis = a ? Date.now() - a.recu : 0;
  return { a, fige: a && depuis >= figeMs() ? Math.floor(depuis / 60_000) : null, fluxOuvert };
}

/// L'avancement en mots : « vecteurs 60 % (600/1000 morceaux) · 410 morceaux/s ».
export function avancementText(a: Avancement, fige: number | null, ouvert: boolean): string {
  const debit = a.debit != null && fige == null ? ` · ${a.debit} ${a.unite}/s` : "";
  const f = fige != null ? ` · figé depuis ${fige} min` : "";
  const coupe = ouvert ? "" : " · flux coupé";
  return `${a.phase} ${a.pourcent} % (${a.fait}/${a.total} ${a.unite})${debit}${f}${coupe}`;
}
