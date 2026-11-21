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
}

function run({ inputs, config }: { inputs: { work: Work }; config: { style: string } }) {
  const w = inputs.work;
  const last = w.last_text.trim();
  if (w.read.length === 0 && w.wrote.length === 0 && w.command === null && last === "") {
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
  const chars = Array.from(b);
  if (chars.length > w.budget) {
    const cut = chars.slice(0, Math.max(0, w.budget - 80)).join("");
    b = `${cut}\n[Previous step cut at ${w.budget} characters: teammate.carry_over_chars]\n`;
  }
  return { block: b };
}
