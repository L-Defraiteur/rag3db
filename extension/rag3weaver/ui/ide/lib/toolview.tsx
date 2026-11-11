import { useState } from "react";

// **Le retour d'un outil, lisible par un humain** (ticket du 7 novembre 2026, « les
// retours d'outils du chat n'ont pas de mise en page humaine »). La règle de Lucie :
// « le texte ne doit pas différer, on doit savoir quand il manque une info, juste la
// mise en page peut changer ». Le rendu part du texte que le modèle a reçu, sans rien
// en retirer : le Markdown du résultat habillé (les lignes numérotées de read_file
// colorées), tout le reste en « autres champs », et le bouton « brut » qui montre le
// texte exact. Un texte coupé (la borne des retours d'outils) le dit.

/// Ce que le modèle a reçu, lu : le texte à habiller, les autres champs, et s'il est
/// coupé.
export type ToolText = { markdown: string | null; extras: Record<string, unknown> | null; truncated: boolean };

const COUPURE = /\n\.\.\. \d+ more lines \(output truncated\)\n?$/;

export function readToolText(brut: string): ToolText {
  const truncated = COUPURE.test(brut);
  // search_code : le texte de présentation, puis « Metadata: {json} ».
  const meta = brut.lastIndexOf("\nMetadata: {");
  if (!brut.trimStart().startsWith("{")) {
    if (meta >= 0) {
      try {
        return { markdown: brut.slice(0, meta), extras: { metadata: JSON.parse(brut.slice(meta + "\nMetadata: ".length)) }, truncated };
      } catch {
        /* la métadonnée coupée : tout reste dans le texte */
      }
    }
    return { markdown: brut, extras: null, truncated };
  }
  // read_file, grep_files, list_files : la réponse entière en JSON, le Markdown dans `result`.
  try {
    const v = JSON.parse(brut) as Record<string, unknown>;
    const { result, ...extras } = v;
    return { markdown: typeof result === "string" ? result : null, extras: typeof result === "string" ? extras : v, truncated };
  } catch {
    return { markdown: null, extras: null, truncated: true };
  }
}

/// La cible d'une action vient du reçu structuré, jamais du texte du modèle.
/// Les anciens outils locaux gardent le contexte de leur conversation.
export function toolContext(args: string, metadata: unknown, localId: string): { id: string | null; label?: string } {
  const object = (v: unknown): Record<string, unknown> | null => v !== null && typeof v === "object" && !Array.isArray(v) ? v as Record<string, unknown> : null;
  const meta = object(metadata);
  const invocation = object(meta?.invocation);
  let requested: Record<string, unknown> | null;
  try { requested = object(JSON.parse(args)); } catch { return { id: null }; }
  if (!invocation && !Object.prototype.hasOwnProperty.call(requested ?? {}, "context") && !Object.prototype.hasOwnProperty.call(meta ?? {}, "invocation")) return { id: localId };
  const target = object(invocation?.target);
  const context = object(requested?.context);
  if (!target || target.kind !== "backend" || typeof target.id !== "string" || !target.id.trim()
      || typeof target.label !== "string" || !target.label.trim()
      || typeof invocation?.parent_run !== "string" || !invocation.parent_run
      || typeof invocation.run !== "string" || !invocation.run
      || invocation.view != null || context?.view != null) return { id: null };
  return { id: target.id, label: target.label };
}

/// Le retour d'un outil : le rendu humain, et le brut à un clic.
export function ToolResultView({ text, render }: { text: string; render: (markdown: string) => string }) {
  const [brut, setBrut] = useState(false);
  const lu = readToolText(text);
  return (
    <div className="retour-outil mt-2">
      <div className="flex justify-end">
        <button type="button" className="brut text-[11px] underline" onClick={() => setBrut((b) => !b)} aria-pressed={brut}>
          {brut ? "mise en page" : "brut"}
        </button>
      </div>
      {brut || lu.markdown === null ? (
        <pre className="resultat texte-brut whitespace-pre-wrap break-words max-h-[420px] overflow-auto text-xs">{text || "(aucun résultat)"}</pre>
      ) : (
        <div className="resultat rendu-humain markdown text-xs max-h-[520px] overflow-auto" dangerouslySetInnerHTML={{ __html: render(lu.markdown) }} />
      )}
      {!brut && lu.extras && Object.keys(lu.extras).length ? (
        <details className="autres-champs text-[11px] mt-1">
          <summary className="cursor-pointer">autres champs ({Object.keys(lu.extras).join(", ")})</summary>
          <pre className="whitespace-pre-wrap break-words">{JSON.stringify(lu.extras, null, 2)}</pre>
        </details>
      ) : null}
      {lu.truncated ? <p className="coupe text-[11px] text-amber-700 dark:text-amber-400">le texte reçu par le modèle est coupé à sa borne : ce qui manque ne s'affiche pas</p> : null}
    </div>
  );
}
