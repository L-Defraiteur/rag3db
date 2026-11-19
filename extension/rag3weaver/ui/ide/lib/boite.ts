// How a mailbox line reads in the IDE (mode pair, lot 4, page 05 of 19 Nov 2026,
// rag3weaver-archi): a session's message shows who wrote it and its text without the
// <cross-session-message> envelope; a message sent to a session shows to whom.

export type LigneVue = {
  /** Who wrote it: the session's name, the teammate, the supervisor, the host. */
  qui: string;
  /** To whom, for a message sent to a session. */
  vers?: string;
  /** The line's kind (request, report, message…). */
  genre: string;
  /** The text to read. */
  texte: string;
  /** A line of the bus between sessions. */
  session: boolean;
};

const ENVELOPPE = /^<cross-session-message([^>]*)>\n?([\s\S]*?)\n?<\/cross-session-message>\s*$/;

export function ligneDeBoite(l: Record<string, unknown>): LigneVue {
  const texte = String(l.text ?? "");
  const genre = String(l.kind ?? "");
  if (l.from === "session") {
    const m = ENVELOPPE.exec(texte);
    const nom = m ? /from-name="([^"]*)"/.exec(m[1])?.[1] : undefined;
    return { qui: String(l.peer || nom || "session"), genre, texte: m ? m[2] : texte, session: true };
  }
  if (l.from === "teammate" && genre === "message" && l.direction === "out") {
    return { qui: "teammate", vers: String(l.to ?? ""), genre, texte, session: true };
  }
  return { qui: String(l.from ?? ""), genre, texte, session: false };
}
