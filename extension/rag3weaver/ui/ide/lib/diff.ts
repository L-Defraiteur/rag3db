// **La différence ligne à ligne** (la plus longue sous-suite commune), telle que
// l'ancienne page la rendait : une ligne `-` par ligne d'avant retirée, `+` par
// ligne d'après ajoutée ; au-delà de quatre millions de paires, on renonce.
export type DiffLine = { kind: "plus" | "moins"; text: string };

export function lineDiff(before: string, after: string): DiffLine[] | string {
  const a = before.split("\n");
  const b = after.split("\n");
  if (a.length * b.length > 4e6) return "fichiers trop grands pour la différence ligne à ligne : reprenez le disque ou écrasez en connaissance de cause";
  const l: Uint32Array[] = Array.from({ length: a.length + 1 }, () => new Uint32Array(b.length + 1));
  for (let i = a.length - 1; i >= 0; i--) for (let j = b.length - 1; j >= 0; j--) l[i][j] = a[i] === b[j] ? l[i + 1][j + 1] + 1 : Math.max(l[i + 1][j], l[i][j + 1]);
  const out: DiffLine[] = [];
  let i = 0;
  let j = 0;
  while (i < a.length || j < b.length) {
    if (i < a.length && j < b.length && a[i] === b[j]) {
      i++;
      j++;
      continue;
    }
    if (i < a.length && (j >= b.length || l[i + 1][j] >= l[i][j + 1])) out.push({ kind: "moins", text: "- " + a[i++] });
    else out.push({ kind: "plus", text: "+ " + b[j++] });
  }
  return out.length ? out : "aucune différence de lignes (fins de ligne ou espaces)";
}
