import assert from "node:assert/strict";
import { ligneDeBoite } from "./boite.ts";

// A session's message: who wrote it, and its text without the envelope.
const recue = {
  from: "session", kind: "request", peer: "rag3db-5a", reply_to_address: "uds:/run/user/1000/cc-socks/42.sock",
  text: '<cross-session-message from="uds:/run/user/1000/cc-socks/42.sock" from-name="rag3db-5a" from-mode="prompting">\nWhat is 2+2?\n</cross-session-message>',
};
assert.deepEqual(ligneDeBoite(recue), { qui: "rag3db-5a", genre: "request", texte: "What is 2+2?", session: true });
// Without peer, the name of the envelope; without envelope, the text as it is.
assert.equal(ligneDeBoite({ ...recue, peer: undefined }).qui, "rag3db-5a");
assert.equal(ligneDeBoite({ ...recue, text: "plain" }).texte, "plain");
assert.equal(ligneDeBoite({ ...recue, peer: undefined, text: "plain" }).qui, "session");
// A message sent to a session: from the teammate, to whom.
const envoyee = { from: "teammate", kind: "message", direction: "out", to: "rag3db-5a", text: "4" };
assert.deepEqual(ligneDeBoite(envoyee), { qui: "teammate", vers: "rag3db-5a", genre: "message", texte: "4", session: true });
// Every other line as before.
assert.deepEqual(ligneDeBoite({ from: "supervisor", kind: "request", text: "Fais-le." }), { qui: "supervisor", genre: "request", texte: "Fais-le.", session: false });
assert.deepEqual(ligneDeBoite({ from: "teammate", kind: "report", text: "fait" }), { qui: "teammate", genre: "report", texte: "fait", session: false });
console.log("VERT : les lignes du bus entre sessions se lisent qui → qui, sans enveloppe ; les autres comme avant");
