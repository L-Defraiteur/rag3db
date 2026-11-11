import assert from "node:assert/strict";
import { build } from "esbuild";

// La frontière d'autorité ne dépend pas de la présentation du modèle.
const built = await build({ entryPoints: ["lib/toolview.tsx"], bundle: true, platform: "node", format: "esm", write: false });
const { toolContext } = await import(`data:text/javascript;base64,${Buffer.from(built.outputFiles[0].text).toString("base64")}`);
const request = JSON.stringify({ context: { target: { kind: "backend", id: "requested" } } });
const receipt = { invocation: { target: { kind: "backend", id: "effective", label: "Dépôt effectif" }, parent_run: "parent", run: "child" } };
assert.deepEqual(toolContext('{"path":"src/lib.rs"}', undefined, "local"), { id: "local" });
assert.deepEqual(toolContext(request, receipt, "local"), { id: "effective", label: "Dépôt effectif" });
// Ancienne session, reçu inconnu, vue non prise en charge : aucun repli local.
for (const metadata of [undefined, {}, { invocation: {} }, { invocation: { ...receipt.invocation, target: { kind: "space", id: "x" } } }, { invocation: { ...receipt.invocation, view: { branch: "main" } } }]) {
  assert.equal(toolContext(request, metadata, "local").id, null);
}
assert.equal(toolContext(JSON.stringify({ context: { view: "inconnue" } }), receipt, "local").id, null);
assert.equal(toolContext("{", receipt, "local").id, null);
console.log("VERT : contexte local, reçu autoritaire, absence et vues inconnues sans repli");
