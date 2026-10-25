// archive_summary — la fin d'une section rangée (M4) : vérifie le rapport d'AddRefNode reçu par le
// port (la provenance doc: est posée), et rend {title, kind, fiche, source, section}.
// input : #{ element: l'élément décidé, value: le rapport d'AddRefNode }
let e = input.element;
let r = input.value;
if r.executed != true { throw `archive : la provenance de « ${e.title} » n'est pas posée : ${r.why}`; }
#{ title: e.title, kind: e.kind, fiche: e.fiche, source: e.chemin, section: e.title, verdict: e.verdict }
