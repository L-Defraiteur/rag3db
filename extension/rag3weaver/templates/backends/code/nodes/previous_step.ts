// Le bloc « Previous step » de l'équipier, en script : la même sortie que StepWork::block (Rust) en style « full ».
// Les longueurs se comptent en caractères Unicode, comme `chars()` en Rust : Array.from parcourt les points de code,
// là où `.length` compterait des unités UTF-16 (un emoji en vaut deux).

interface Work {
  number: number;
  read: string[];
  wrote: string[];
  command: { command: string; output: string } | null;
  last_text: string;
  budget: number;
  results?: { tool: string; result: string }[];
  previous_block?: string;
}

// The facts a step's index tools gave: their lines that name a place (`path:line`), without a file's header scope,
// taken in turn from each result so that every file mapped gives its share, at most `per_file` per file; then the
// facts of the block this step received, after them, so that a fact of an earlier step stays while there is room.
function placeOf(line: string): string {
  const m = line.match(/([\w.\/-]+\.[\w]+):\d+/);
  return m ? m[1] : "";
}

function facts(w: Work, tools: string[], max: number, perFile: number): string[] {
  const lists: string[][] = [];
  for (const r of w.results ?? []) {
    if (!tools.includes(r.tool)) continue;
    const l: string[] = [];
    for (const raw of r.result.split("\n")) {
      const line = raw.replace(/^[\s\-*•]+/, "").trim();
      if (!/[\w.\/-]+\.[\w]+:\d+/.test(line) || /`file_scope_\d+`/.test(line)) continue;
      l.push(Array.from(line).slice(0, 240).join(""));
    }
    if (l.length > 0) lists.push(l);
  }
  const seen = new Set<string>();
  const byFile = new Map<string, number>();
  const out: string[] = [];
  const take = (line: string): boolean => {
    if (seen.has(line)) return false;
    const f = placeOf(line);
    if ((byFile.get(f) ?? 0) >= perFile) return false;
    seen.add(line);
    byFile.set(f, (byFile.get(f) ?? 0) + 1);
    out.push(line);
    return true;
  };
  for (let i = 0; out.length < max && lists.some((l) => i < l.length); i++) {
    for (const l of lists) {
      if (out.length >= max) break;
      if (i < l.length) take(l[i]);
    }
  }
  const prev = w.previous_block ?? "";
  const at = prev.indexOf("Found by the code index");
  if (at >= 0) {
    for (const raw of prev.slice(at).split("\n").slice(1)) {
      if (out.length >= max) break;
      if (!raw.startsWith("- ")) continue;
      take(raw.slice(2).trim());
    }
  }
  return out;
}

function run({ inputs, config }: { inputs: { work: Work }; config: { style: string; fact_tools?: string[]; max_facts?: number; max_facts_per_file?: number } }) {
  const w = inputs.work;
  const last = w.last_text.trim();
  const found = facts(w, config.fact_tools ?? ["search_code", "usages", "callees", "impact", "outline", "grep_files"], config.max_facts ?? 40, config.max_facts_per_file ?? 12);
  if (w.read.length === 0 && w.wrote.length === 0 && w.command === null && last === "" && found.length === 0) {
    return { block: "" };
  }
  let b = `\n## Previous step (step ${w.number})\n`;
  if (w.read.length > 0) {
    b += `Read: ${w.read.join(", ")}\n`;
  }
  if (w.wrote.length > 0) {
    b += `Wrote: ${w.wrote.join(", ")}\n`;
  }
  if (w.command !== null) {
    b += config.style === "compact"
      ? `Last command: ${w.command.command}\n`
      : `Last command: ${w.command.command}\nIts output ends with:\n${w.command.output}\n`;
  }
  if (last !== "") {
    b += `Your last text: ${last}\n`;
  }
  b += "Lines you read in that step can be replaced now without reading them again.\n";
  // The index's facts, kept whole: over the budget, the prose is cut before them (they are what stops an invented name).
  const kept = found.length === 0
    ? ""
    : `Found by the code index in that step (cite these names and places exactly; a name that is not here nor in a tool result of this step does not exist for you):\n${found.map((f) => `- ${f}\n`).join("")}`;
  const room = w.budget - Array.from(kept).length;
  const chars = Array.from(b);
  if (chars.length > room) {
    const cut = chars.slice(0, Math.max(0, room - 80)).join("");
    b = `${cut}\n[Previous step cut at ${w.budget} characters: teammate.carry_over_chars]\n`;
  }
  return { block: b + kept };
}
