import { useEffect, useState } from "react";
import { Scale } from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { shortName, type HostState } from "@/lib/host";

// **Les bilans** (page 08, vue 6 ; pages 04 et 05) : ce que le juge de session a
// rangé dans la mémoire — par fiche de tâche, ses apports dans l'ordre et la
// tendance des notes ; et la fiche de chaque outil : la tendance par genre de
// défaut run après run (un défaut qui revient est le signal qu'on change l'outil),
// les `autre` groupés par libellé (le candidat au prochain genre), les défauts du
// dernier run par gravité. Repères de l'ancienne page gardés : `#bilans-absents`,
// `section.fiche-de-bilan[data-fiche]`, `.tendance`, `details.apport-de-bilan`,
// `section.fiche-d-outil[data-fiche]`, `.tendance-par-genre li[data-genre]`,
// `.autres`, `.gravite[data-gravite]`.
type Defect = { genre: string; libelle?: string; call_id?: string; preuve?: string; proposee?: string; ce_que_l_agent_en_a_fait?: string };
type Apport = { source?: string; creee?: number; texte: string; note?: number | null; comparaison?: { verdict?: string }; defauts?: Record<string, Defect[]> | null; part_utilisee?: number | null };
type Fiche = { fiche: string; outil: boolean; about?: string; apports: Apport[] };
const GRAVITIES = ["bloquant", "genant", "mineur"] as const;
const genreOf = (d: Defect) => (d.genre === "autre" ? `autre — ${d.libelle ?? ""}` : d.genre);

function SessionFiche({ f }: { f: Fiche }) {
  const last = f.apports.at(-1);
  return (
    <section className="fiche-de-bilan rounded-lg border px-3 py-2 grid gap-2" data-fiche={f.fiche}>
      <h2 className="text-base font-semibold">{f.fiche}</h2>
      <p className="note tendance text-sm text-muted-foreground">
        notes : {f.apports.map((a) => a.note ?? "?").join(" → ")}
        {f.apports.length > 1 && last?.comparaison?.verdict ? ` · ${last.comparaison.verdict}` : ""}
      </p>
      {f.apports.map((a, k) => (
        <details key={k} className="apport-de-bilan rounded border px-3 py-1.5 text-sm">
          <summary className="cursor-pointer">note {a.note ?? "?"} · {a.source ?? ""}</summary>
          <pre className="whitespace-pre-wrap break-words text-xs mt-2 text-foreground/90">{a.texte}</pre>
        </details>
      ))}
    </section>
  );
}

function ToolFiche({ f }: { f: Fiche }) {
  const byGenre = new Map<string, number[]>();
  const others = new Map<string, number>();
  f.apports.forEach((a, i) => {
    for (const g of GRAVITIES)
      for (const d of a.defauts?.[g] ?? []) {
        const key = genreOf(d);
        if (!byGenre.has(key)) byGenre.set(key, Array(f.apports.length).fill(0));
        byGenre.get(key)![i] += 1;
        if (d.genre === "autre") others.set(d.libelle ?? "", (others.get(d.libelle ?? "") ?? 0) + 1);
      }
  });
  const last = f.apports.at(-1);
  return (
    <section className="fiche-d-outil rounded-lg border px-3 py-2 grid gap-2" data-fiche={f.fiche}>
      <h2 className="text-base font-semibold">{f.fiche}</h2>
      <ul className="tendance-par-genre text-sm grid gap-0.5">
        {[...byGenre.entries()].map(([genre, n]) => (
          <li key={genre} data-genre={genre}>
            <code>{genre}</code> : {n.join(" → ")}
          </li>
        ))}
      </ul>
      {others.size ? <p className="note autres text-xs text-muted-foreground">autres, par libellé : {[...others.entries()].map(([l, n]) => `${l} (${n})`).join(", ")}</p> : null}
      {last
        ? GRAVITIES.map((g) => {
            const list = last.defauts?.[g] ?? [];
            return (
              <div key={g} className={`gravite ${g}`} data-gravite={g}>
                <h3 className="text-sm font-semibold flex items-center gap-2">
                  {g} <Badge variant={g === "bloquant" ? "destructive" : g === "genant" ? "secondary" : "outline"}>{list.length}</Badge>
                </h3>
                <ul className="list-disc pl-5 text-sm">
                  {list.map((d, k) => (
                    <li key={k}>
                      <code>{genreOf(d)}</code> (appel {d.call_id}) : « {d.preuve} » → {d.proposee}
                    </li>
                  ))}
                </ul>
              </div>
            );
          })
        : null}
      {f.apports.map((a, k) => (
        <details key={k} className="apport-de-bilan rounded border px-3 py-1.5 text-sm">
          <summary className="cursor-pointer">{a.source ?? ""}</summary>
          <pre className="whitespace-pre-wrap break-words text-xs mt-2 text-foreground/90">{a.texte}</pre>
        </details>
      ))}
    </section>
  );
}

export function Bilans({ id, state }: { id: string; state: HostState }) {
  const backend = state.backends.find((b) => b.id === id);
  const [r, setR] = useState<{ declare: boolean; fiches: Fiche[] } | null>(null);
  const [error, setError] = useState<string | null>(null);
  useEffect(() => {
    fetch(`/ide/bilans?${new URLSearchParams({ backend: id })}`)
      .then((x) => x.json())
      .then((x) => (x.ok ? setR(x.result) : setError(x.error)))
      .catch((e: Error) => setError(e.message));
  }, [id]);
  if (!backend) return <p className="text-destructive">backend inconnu : {id}</p>;
  const fiches = r?.fiches.filter((f) => !f.outil) ?? [];
  const tools = r?.fiches.filter((f) => f.outil) ?? [];
  return (
    <>
      <p id="fil" className="text-xs text-muted-foreground mb-3">
        <a href="/ide/" className="hover:underline">backends</a> · {shortName(backend.manifest)} · bilans
      </p>
      <h1 className="text-xl font-semibold mb-1 inline-flex items-center gap-2"><Scale className="size-5" /> Bilans de session</h1>
      {error ? <p className="text-destructive" role="alert">{error}</p> : null}
      {r ? (
        <div className="grid gap-3 mt-2">
          {r.declare ? (
            <p className="note text-sm text-muted-foreground">Le juge relit chaque tour du chat de ce backend, et range ici son bilan, par tâche.</p>
          ) : (
            <p className="note text-sm text-muted-foreground" id="bilans-absents">Ce backend ne déclare pas `bilans` : ses tours ne sont pas jugés.</p>
          )}
          {r.declare && !fiches.length ? <p className="note text-sm text-muted-foreground">aucun bilan encore</p> : null}
          {fiches.map((f) => <SessionFiche key={f.fiche} f={f} />)}
          {tools.length ? <h1 className="text-xl font-semibold mt-4">Les outils</h1> : null}
          {tools.map((f) => <ToolFiche key={f.fiche} f={f} />)}
        </div>
      ) : error ? null : (
        <p className="note text-muted-foreground">Chargement…</p>
      )}
    </>
  );
}
