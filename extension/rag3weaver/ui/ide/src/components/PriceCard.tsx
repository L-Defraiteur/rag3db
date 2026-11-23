import { useState } from "react";
import { CircleDollarSign, Search, PencilLine } from "lucide-react";
import { Button } from "@/components/ui/button";
import { lookupPrice, setPrice, type PriceProposal } from "@/lib/host";

// **A model without a price, as a gesture** (Lucie, 22 November 2026: « un refus offre le geste »; point 10 of the IDE's
// design ideas). The refusal says the model in one sentence and offers to do it: « je le cherche » (the host looks the
// price up on its provider's page, and shows it with its source for a yes) or « tu le donnes » (three fields). The price
// is written by the host on the user's yes only; then the message that was refused goes again by itself. Markers of a
// witness: `.price-card[data-slug]`, `[data-step]`.

type Step = "ask" | "looking" | "proposal" | "giving" | "writing";

const num = (v: string): number | null => {
  const n = Number(v.replace(",", "."));
  return v.trim() === "" || !Number.isFinite(n) || n < 0 ? null : n;
};

export function PriceCard({ slug, onWritten, provisional }: { slug: string; onWritten: () => void; provisional?: PriceProposal & { note?: string } }) {
  // A provisional price (set by the host to go on) opens on its proposal: confirm or correct, nothing stopped.
  const [step, setStep] = useState<Step>(provisional ? "proposal" : "ask");
  const [proposal, setProposal] = useState<PriceProposal | null>(provisional ?? null);
  const [given, setGiven] = useState({ input: "", output: "", cache_read: "" });
  const [error, setError] = useState<string | null>(null);

  const look = async () => {
    setError(null);
    setStep("looking");
    try {
      setProposal(await lookupPrice(slug));
      setStep("proposal");
    } catch (e) {
      setError(`Je n'ai pas trouvé son prix : ${(e as Error).message}`);
      setStep("ask");
    }
  };

  const write = async (price: PriceProposal) => {
    setError(null);
    const before = step;
    setStep("writing");
    try {
      await setPrice(slug, price);
      onWritten();
    } catch (e) {
      setError((e as Error).message);
      setStep(before);
    }
  };

  const givenPrice = (): PriceProposal | null => {
    const input = num(given.input);
    const output = num(given.output);
    if (input === null || output === null) return null;
    return { input, output, cache_read: num(given.cache_read), source: "given by the user" };
  };

  return (
    <article className="price-card mb-7 rounded-lg border border-primary/50 px-3.5 py-3 text-sm grid gap-2.5" data-slug={slug} data-step={step}>
      <p className="flex items-start gap-2">
        <CircleDollarSign className="size-4 mt-0.5 shrink-0 text-primary" />
        {provisional ? (
          <span>
            Le modèle <code>{slug}</code> n'avait pas de prix : j'ai pris un prix provisoire pour continuer
            {provisional.note ? ` (${provisional.note})` : ""}. Tu le confirmes, ou tu le corriges ?
          </span>
        ) : (
          <span>
            Le modèle <code>{slug}</code> n'a pas de prix, et le plafond des dépenses ne sait pas le compter. Je le cherche
            sur la page de son fournisseur, ou tu le donnes ?
          </span>
        )}
      </p>
      {step === "ask" || step === "looking" ? (
        <div className="flex flex-wrap gap-2">
          <Button size="sm" onClick={look} disabled={step === "looking"} data-action="look">
            <Search className="size-4" /> {step === "looking" ? "Je cherche…" : "Cherche-le"}
          </Button>
          <Button size="sm" variant="outline" onClick={() => setStep("giving")} disabled={step === "looking"} data-action="give">
            <PencilLine className="size-4" /> Je le donne
          </Button>
        </div>
      ) : null}
      {step === "proposal" && proposal ? (
        <div className="grid gap-2" data-proposal>
          <p>
            Entrée {proposal.input} $, sortie {proposal.output} $
            {proposal.cache_read != null ? `, lecture du cache ${proposal.cache_read} $` : ""} par million de jetons
            {proposal.source ? (
              <>
                {" "}
                — lu sur{" "}
                <a href={proposal.source} target="_blank" rel="noreferrer" className="underline">
                  {proposal.source}
                </a>
              </>
            ) : null}
            .
          </p>
          <div className="flex flex-wrap gap-2">
            <Button size="sm" onClick={() => write({ ...proposal, source: proposal.source || "confirmed by the user" })} data-action="accept">
              {provisional ? "Confirmer" : "C'est bon, écris-le"}
            </Button>
            <Button size="sm" variant="outline" onClick={() => setStep("giving")} data-action="give">
              {provisional ? "Corriger" : "Je le donne moi-même"}
            </Button>
          </div>
        </div>
      ) : null}
      {step === "giving" || (step === "writing" && !proposal) ? (
        <div className="grid gap-2" data-giving>
          <div className="flex flex-wrap gap-2 items-end">
            {(
              [
                ["input", "Entrée"],
                ["output", "Sortie"],
                ["cache_read", "Lecture du cache (facultatif)"],
              ] as const
            ).map(([key, label]) => (
              <label key={key} className="grid gap-1 text-xs text-muted-foreground">
                {label}, $ par million
                <input
                  className="w-32 rounded border bg-background px-2 py-1 text-sm text-foreground"
                  inputMode="decimal"
                  value={given[key]}
                  data-field={key}
                  onChange={(e) => setGiven({ ...given, [key]: e.target.value })}
                />
              </label>
            ))}
            <Button
              size="sm"
              disabled={!givenPrice() || step === "writing"}
              onClick={() => {
                const p = givenPrice();
                if (p) void write(p);
              }}
              data-action="write"
            >
              Écrire ce prix
            </Button>
          </div>
        </div>
      ) : null}
      {step === "writing" ? <p className="text-xs text-muted-foreground">J'écris le prix…</p> : null}
      {error ? <p className="text-xs text-destructive">{error}</p> : null}
    </article>
  );
}
