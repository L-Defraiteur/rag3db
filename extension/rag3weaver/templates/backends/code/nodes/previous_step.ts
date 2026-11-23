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
}

// The facts a step's index tools gave: their lines that name a place (`path:line`), each once, in order. A file's
// content read by another tool is no fact of the index.
function facts(w: Work, tools: string[], max: number): string[] {
  const seen = new Set<string>();
  const out: string[] = [];
  for (const r of w.results ?? []) {
    if (!tools.includes(r.tool)) continue;
    for (const raw of r.result.split("\n")) {
      const line = raw.replace(/^[\s\-*•]+/, "").trim();
      if (!/[\w.\/-]+\.[\w]+:\d+/.test(line) || seen.has(line)) continue;
      seen.add(line);
      out.push(Array.from(line).slice(0, 240).join(""));
      if (out.length >= max) return out;
    }
  }
  return out;
}

function run({ inputs, config }: { inputs: { work: Work }; config: { style: string; fact_tools?: string[]; max_facts?: number } }) {
  const w = inputs.work;
  const last = w.last_text.trim();
  const found = facts(w, config.fact_tools ?? ["search_code", "usages", "callees", "impact", "outline", "grep_files"], config.max_facts ?? 40);
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
