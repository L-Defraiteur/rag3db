import { useEffect, useRef, useState } from "react";
import { Save, X } from "lucide-react";
import { Button } from "@/components/ui/button";
import { lineDiff } from "@/lib/diff";
import { call, onFlux, type FluxEvent } from "@/lib/host";

// **L'édition d'un fichier** (page 08, vue 3b ; B1, page 05 §4) : CodeMirror par
// le magasin de libs embarqué dans l'hôte (`/ide/registre.js`, `/ide/editeur/
// modules.js`, rangé par scripts/ide_libs.sh) — ni CDN, ni téléchargement à
// l'usage ; une zone de texte le remplace s'il ne se charge pas, en le disant.
// Ctrl-S (Cmd-S) enregistre sur l'empreinte lue (`expected_hash`) ; si le fichier
// a changé depuis, rien n'est écrit : la différence disque → tampon se montre, et
// la personne choisit (reprendre le disque, ou écraser en le sachant). Le tampon
// n'est jamais remplacé sans son choix. Repères du témoin : `.edition .cm-content`,
// `.barre .note` (« modifié, non enregistré », « enregistré », « changé sur le
// disque… », « repris du disque »), `.diff .plus|.moins`, le bouton « Garder ma
// version… ».
type Zone = { value: string; focus(): void };
type CmModules = { view: any; state: any; commands: any; search: any; language: any };
let modules: Promise<CmModules> | null = null;

function loadScript(src: string): Promise<void> {
  return new Promise((ok, ko) => {
    const s = document.createElement("script");
    s.src = src;
    s.onload = () => ok();
    s.onerror = () => ko(new Error(`${src} ne se charge pas`));
    document.head.append(s);
  });
}
function editorModules(): Promise<CmModules> {
  if (!modules) {
    modules = (async () => {
      const g = globalThis as unknown as { __rag3weaver?: { require: (k: string) => any; demarrage: Record<string, string> } };
      if (!g.__rag3weaver) await loadScript("/ide/registre.js");
      await loadScript("/ide/editeur/modules.js");
      const R = g.__rag3weaver!;
      const lib = (spec: string) => R.require(R.demarrage[spec]);
      return { view: lib("@codemirror/view"), state: lib("@codemirror/state"), commands: lib("@codemirror/commands"), search: lib("@codemirror/search"), language: lib("@codemirror/language") };
    })();
  }
  return modules;
}

export function Editor({ manifest, path, onClose }: { manifest: string; path: string; onClose: () => void }) {
  const container = useRef<HTMLDivElement>(null);
  const zone = useRef<Zone | null>(null);
  const hash = useRef<string>("");
  const modified = useRef(false);
  const [notice, setNotice] = useState("");
  const [outside, setOutside] = useState(false);
  const [conflict, setConflict] = useState<{ disk: string; diskHash: string } | null>(null);
  const [ready, setReady] = useState(false);

  const read = () => call(manifest, "read_file", { path, format: "brut" }) as Promise<{ content: string; content_hash: string; outside?: boolean }>;

  const save = async () => {
    const z = zone.current;
    if (!z) return;
    try {
      await call(manifest, "edit_file", { path, content: z.value, expected_hash: hash.current });
    } catch (err) {
      const why = String((err as Error).message ?? err);
      if (why.includes("fichier changé depuis la lecture")) {
        const r = await read();
        setConflict({ disk: r.content, diskHash: r.content_hash });
        setNotice(why);
        return;
      }
      setNotice(`refusé : ${why}`);
      return;
    }
    // Écrit : le tampon est l'état du disque, son empreinte se relit.
    const r = await read();
    hash.current = r.content_hash;
    modified.current = false;
    setConflict(null);
    setNotice("enregistré");
  };
  const saveRef = useRef(save);
  saveRef.current = save;

  useEffect(() => {
    let cancelled = false;
    (async () => {
      const r = await read();
      if (cancelled) return;
      if (r.outside) {
        setOutside(true);
        return;
      }
      hash.current = r.content_hash;
      const mark = () => {
        modified.current = true;
        setNotice("modifié, non enregistré");
      };
      const parent = container.current!;
      try {
        const cm = await editorModules();
        if (cancelled) return;
        const { EditorView, keymap, lineNumbers, highlightActiveLine, highlightActiveLineGutter, drawSelection } = cm.view;
        const { EditorState } = cm.state;
        const { defaultKeymap, history, historyKeymap, indentWithTab } = cm.commands;
        const { searchKeymap, highlightSelectionMatches } = cm.search;
        const { bracketMatching, indentOnInput } = cm.language;
        let fromPage = false;
        const view = new EditorView({
          parent,
          state: EditorState.create({
            doc: r.content,
            extensions: [
              lineNumbers(), highlightActiveLineGutter(), highlightActiveLine(), drawSelection(),
              history(), bracketMatching(), indentOnInput(), highlightSelectionMatches(),
              keymap.of([{ key: "Mod-s", preventDefault: true, run: () => { saveRef.current(); return true; } }, ...defaultKeymap, ...historyKeymap, ...searchKeymap, indentWithTab]),
              EditorView.updateListener.of((u: any) => { if (u.docChanged && !fromPage) mark(); }),
              EditorView.theme({ "&": { height: "70vh" }, ".cm-scroller": { fontFamily: "ui-monospace, monospace", fontSize: "13px" } }),
            ],
          }),
        });
        zone.current = {
          get value() { return view.state.doc.toString(); },
          set value(t: string) { fromPage = true; try { view.dispatch({ changes: { from: 0, to: view.state.doc.length, insert: t } }); } finally { fromPage = false; } },
          focus() { view.focus(); },
        };
      } catch (err) {
        // Sans ses modules, l'édition reste possible : une zone de texte, et la raison dite.
        console.error(`l'éditeur ne se charge pas, une zone de texte le remplace : ${(err as Error).message ?? err}`);
        modules = null;
        const ta = document.createElement("textarea");
        ta.spellcheck = false;
        ta.setAttribute("aria-label", `contenu de ${path}`);
        ta.value = r.content;
        ta.className = "w-full h-[70vh] font-mono text-[13px] rounded-md border bg-input/30 p-2";
        ta.addEventListener("input", mark);
        ta.addEventListener("keydown", (e) => { if ((e.ctrlKey || e.metaKey) && e.key === "s") { e.preventDefault(); saveRef.current(); } });
        parent.append(ta);
        zone.current = { get value() { return ta.value; }, set value(t: string) { ta.value = t; }, focus() { ta.focus(); } };
      }
      setReady(true);
      zone.current?.focus();
    })();
    return () => { cancelled = true; if (container.current) container.current.replaceChildren(); };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [manifest, path]);

  // Le fichier change sur le disque pendant l'édition (un autre éditeur) : un tampon
  // modifié n'est jamais remplacé, on le dit ; un tampon intact se relit.
  useEffect(
    () =>
      onFlux((d: FluxEvent) => {
        const ev = d.evenement ?? {};
        if (d.backend !== manifest || ev.kind !== "FileChanged" || ev.path !== path) return;
        if (ev.hash && ev.hash === hash.current) return; // notre propre enregistrement
        if (modified.current) setNotice("changé sur le disque depuis la lecture : l'enregistrement montrera la différence");
        else read().then((r) => { if (zone.current) { zone.current.value = r.content; hash.current = r.content_hash; } });
      }),
    [manifest, path],
  );

  const close = () => {
    if (modified.current && !window.confirm("Des modifications ne sont pas enregistrées : les abandonner ?")) return;
    onClose();
  };

  if (outside) return <p className="alerte text-destructive">Ce fichier est hors du worktree déclaré : il se lit, il ne se modifie pas.</p>;
  const diff = conflict ? lineDiff(conflict.disk, zone.current?.value ?? "") : null;
  return (
    <>
      <h1 className="text-xl font-semibold mb-2 break-all">{path}</h1>
      <p className="barre flex flex-wrap items-center gap-2 mb-2">
        <Button size="sm" onClick={save} disabled={!ready}><Save className="size-3.5" /> Enregistrer</Button>
        <Button size="sm" variant="outline" onClick={close}><X className="size-3.5" /> Fermer l'édition</Button>
        <span className="note text-xs text-muted-foreground">{notice}</span>
      </p>
      {conflict ? (
        <>
          <p className="barre flex flex-wrap items-center gap-2 mb-2">
            <Button size="sm" variant="outline" onClick={() => { if (zone.current) zone.current.value = conflict.disk; hash.current = conflict.diskHash; modified.current = false; setConflict(null); setNotice("repris du disque"); }}>Reprendre la version du disque</Button>
            <Button size="sm" variant="destructive" onClick={() => { hash.current = conflict.diskHash; setConflict(null); save(); }}>Garder ma version (écrase la leur)</Button>
          </p>
          <div className="diff font-mono text-xs leading-5 overflow-x-auto rounded-lg border mb-2" aria-label="différence disque → tampon">
            {typeof diff === "string" ? <div className="px-2">{diff}</div> : diff!.map((l, k) => <div key={k} className={`${l.kind} ${l.kind === "plus" ? "bg-green-500/15" : "bg-red-500/15"} whitespace-pre px-2`}>{l.text}</div>)}
          </div>
        </>
      ) : null}
      <div ref={container} className="edition rounded-lg border bg-card overflow-hidden" aria-label={`contenu de ${path}`} />
    </>
  );
}
