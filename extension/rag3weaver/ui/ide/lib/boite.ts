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

export function ligneDeBoite(l: Record<string, unknown>): LigneVue {
  return { qui: String(l.from ?? ""), genre: String(l.kind ?? ""), texte: String(l.text ?? ""), session: false };
}
