#!/usr/bin/env python3
# La relecture de l'ancien modèle (Memory, Subject) en fiches (page 06 §5), par les outils du
# gabarit memory, sur une base ouverte en appel unique par le binaire :
#   relecture.py <backend.json> [ecrire]
# Sans « ecrire » : lit et dit ce qu'elle ferait. Rien n'est retiré, jamais.
import json, subprocess, sys, datetime, os

MANIFESTE = sys.argv[1]
ECRIRE = len(sys.argv) > 2 and sys.argv[2] == "ecrire"
env = dict(os.environ, RAG3WEAVER_EMBED_SERVICE=os.environ.get("RAG3WEAVER_EMBED_SERVICE", "127.0.0.1:7980"),
           RAG3WEAVER_EMBED_CHAR_BUDGET="4096", RAG3WEAVER_GPU_DUTY="70")
proc = subprocess.Popen([os.path.expanduser("~/.local/bin/rag3weaver-backend"), MANIFESTE], stdin=subprocess.PIPE,
                        stdout=subprocess.PIPE, stderr=open(MANIFESTE + ".relecture.err", "w"), text=True, env=env)

def op(v):
    proc.stdin.write(json.dumps(v, ensure_ascii=False) + "\n"); proc.stdin.flush()
    return json.loads(proc.stdout.readline())

def appel(nom, arguments):
    r = op({"op": "call", "name": nom, "arguments": arguments})
    if not r.get("ok"):
        raise SystemExit(f"REFUS {nom} {json.dumps(arguments, ensure_ascii=False)[:200]} : {r.get('error')}")
    return r["result"]

def donnees(r):
    # Le résultat d'un get : {delivery, metadata, result: {data, uuid}}.
    return (r.get("result") or {}).get("data") if isinstance(r, dict) else None

def lignes(outil):
    # `*` : la recherche dense rend tout ce qui est posé (limite 200) ; le plein texte ignore les séparateurs.
    r = appel(outil, {"query": "*", "options": {"limit": 200, "filter_condition": {}}})
    return [x["data"] for x in r.get("result", []) if "data" in x]

sujets = lignes("recall_subjects")
memoires = lignes("recall")
# La recherche ne rend que les champs de retour : la ligne entière (said_as, created_at, uuid) par get_memory.
memoires = [donnees(appel("get_memory", {"record": {"claim": m["claim"], "reach": m["reach"]}})) for m in memoires]
print(f"ancien modèle : {len(sujets)} Subject(s), {len(memoires)} Memory(ies)")
for s in sujets: print("  Subject :", json.dumps({k: s.get(k) for k in ("name", "about", "state")}, ensure_ascii=False))
for m in memoires: print("  Memory  :", json.dumps({k: m.get(k) for k in ("claim", "why", "said_as", "kind", "origin", "reach", "state", "revision", "created_at")}, ensure_ascii=False))
par_nom = {s["name"]: s for s in sujets}

def jour(ms):
    return datetime.datetime.fromtimestamp(ms / 1000, datetime.timezone.utc).strftime("%Y-%m-%d")

# Un L0 par jour de création, une section par Memory.
sources = {}
for m in memoires:
    chemin = f"sources/{jour(m['created_at'])}-ancien-modele.md"
    sources.setdefault(chemin, []).append(m)

fait = {"fiches": 0, "apports": 0, "sautes": 0, "sources": 0}
for chemin, ms in sorted(sources.items()):
    j = chemin.split("/")[1][:10]
    corps = [f"# L'ancien modèle de la mémoire, relu — {j}", "",
             "Ce source est écrit par la relecture de l'ancien modèle (Memory, Subject) en fiches (M4, page 06 §5).",
             "Une section par Memory : la claim, le pourquoi, la provenance. Les lignes d'origine restent dans la base.", ""]
    sections = []
    for m in ms:
        titre = (m.get("said_as") or m["claim"][:60]).strip()
        sujet = par_nom.get(m.get("said_as") or "")
        sections.append((m, titre, sujet))
        corps += [f"## {titre}", "", m["claim"], "", f"Pourquoi : {m.get('why') or '(non dit)'}", "",
                  f"Provenance : Memory `{m.get('_uuid', '?')}` (kind {m.get('kind')}, origin {m.get('origin')}, reach {m.get('reach')}, "
                  f"state {m.get('state')}, revision {m.get('revision')}, created_at {m.get('created_at')})"
                  + (f", rangée sous le Subject « {sujet['name']} »." if sujet else ", sans Subject retrouvé par said_as."), ""]
    contenu = "\n".join(corps)
    print(f"source {chemin} : {len(ms)} section(s)")
    if ECRIRE:
        appel("put_source", {"record": {"chemin": chemin, "titre": f"L'ancien modèle de la mémoire, relu — {j}", "contenu": contenu, "origine": "migration", "via": "relecture"}})
        fait["sources"] += 1
    for m, titre, sujet in sections:
        nom = sujet["name"] if sujet else titre
        about = sujet["about"] if sujet else m["claim"]
        etat = "current" if sujet else "proposed"
        texte = m["claim"] + (f" {m['why']}" if m.get("why") else "")
        # La fiche existe-t-elle, et porte-t-elle déjà ce texte ?
        existante = op({"op": "call", "name": "get_fiche", "arguments": {"record": {"name": nom}}})
        presente = existante.get("ok") and donnees(existante.get("result")) is not None
        deja = False
        if presente:
            apports = appel("recall_apports", {"query": m["claim"][:80], "options": {"limit": 20, "filter_condition": {"field": {"key": "fiche", "value": nom}}}})
            deja = any(x.get("data", {}).get("texte") == texte for x in apports.get("result", []))
        if deja:
            print(f"  « {nom} » : l'apport existe déjà, sauté"); fait["sautes"] += 1; continue
        print(f"  « {nom} » ({etat}) ← L1 section « {titre} »" + ("" if presente else " (fiche à créer)"))
        if not ECRIRE:
            continue
        if not presente:
            appel("put_fiche", {"record": {"name": nom, "about": about, "state": etat, "via": "relecture"}}); fait["fiches"] += 1
        cle = {"fiche": nom, "source": chemin, "section": titre}
        appel("put_apport", {"record": dict(cle, texte=texte, niveau="L1", state="current", via="relecture")})
        appel("relier_apport", {"links": [{"from": {"name": nom}, "to": cle}]})
        r = appel("citer_source", {"genre": "doc", "valeur": f"{chemin}#{titre}", "fiche": cle})
        r = r.get("result", r) if isinstance(r, dict) else {}
        if r.get("executed") is False and "doc" in json.dumps(r.get("why", "")):
            appel("create_ref_type", {"name": "doc", "description": "La section d'un source Markdown, provenance d'un apport : doc:<chemin du source>#<titre de la section>.", "forme": "doc:<chemin du source>#<titre>", "confirm": True})
            r = appel("citer_source", {"genre": "doc", "valeur": f"{chemin}#{titre}", "fiche": cle}).get("result", {})
        if r.get("executed") is not True:
            raise SystemExit(f"REFUS citer_source : {json.dumps(r, ensure_ascii=False)[:300]}")
        fait["apports"] += 1

print("fait :", json.dumps(fait), "(lecture seule)" if not ECRIRE else "")
if ECRIRE:
    for nom in sorted({(par_nom.get(m.get("said_as") or "") or {"name": (m.get("said_as") or m["claim"][:60]).strip()})["name"] for m in memoires}):
        f = appel("get_fiche", {"record": {"name": nom}})
        print("fiche :", json.dumps({k: (donnees(f) or {}).get(k) for k in ("name", "about", "state", "via", "revision")}, ensure_ascii=False))
    for chemin in sources:
        c = appel("cited_by", {"value": chemin, "format": "json"})
        print("cited_by", chemin, ":", json.dumps(json.loads(c) if isinstance(c, str) else c, ensure_ascii=False)[:600])
print("état :", json.dumps(op({"op": "index_state"}).get("result", {}).get("files"), ensure_ascii=False))
print(op({"op": "shutdown"}))
