#!/usr/bin/env python3
"""Sonde LadybugDB — réapplication de nos greffons du cœur sur leur arbre.

Sépare, dans src/, le pur renommage du travail réel, puis tente de reposer
chacun de nos greffons sur l'arbre de LadybugDB par une fusion à trois branches
(base = l'amont Kuzu, « à nous » = notre greffon, « à eux » = leur version), les
deux renommages neutralisés. Classe ensuite les conflits.

LECTURE SEULE. N'utilise que `git show` / `git diff`, n'écrit que dans un dossier
temporaire, ne touche ni à l'index ni à l'arbre de travail.

Deux modes :
  strict    — on neutralise seulement les renommages de nom.
  dialecte  — on traduit en plus nos macros vers les leurs (KU_ASSERT →
              DASSERT, ku_dynamic_cast → dynamic_cast_checked,
              KU_UNREACHABLE → UNREACHABLE_CODE), ce qu'un rebasage ferait
              de toute façon. Donne le compte honnête.

Produit les chiffres du document 05, §2.

    python3 docs/3-septembre-2026-14h42/reapplication.py
"""

import subprocess, os, re, shutil, tempfile

REPO=os.environ.get("RAG3DB_REPO") or subprocess.run(
    ["git","rev-parse","--show-toplevel"],capture_output=True,text=True).stdout.strip()
SB=os.path.join(tempfile.gettempdir(),"sonde-ladybug-reapplication")
UP="89f0263cc"; LB="ladybug-main-2026-08-31"; NS="HEAD"

def git(*a):
    r=subprocess.run(["git","-C",REPO]+list(a),capture_output=True,text=True)
    return r.returncode, r.stdout, r.stderr
def show(rev,path):
    rc,out,_=git("show",f"{rev}:{path}")
    return out if rc==0 else None

def norm_ours(s):
    for a,b in (("RAG3DB","KUZU"),("Rag3db","Kuzu"),("Rag3DB","Kuzu"),("rag3db","kuzu")): s=s.replace(a,b)
    return s
def norm_theirs(s):
    for a,b in (("LADYBUG","KUZU"),("Ladybug","Kuzu"),("ladybug","kuzu"),
                ("LBUG","KUZU"),("Lbug","Kuzu"),("lbug","kuzu")): s=s.replace(a,b)
    return s
MECH=[(r'\bKU_ASSERT\b','DASSERT'),(r'\bku_dynamic_cast\b','dynamic_cast_checked'),
      (r'\bKU_UNREACHABLE\b','UNREACHABLE_CODE')]
def dialect(s):
    for a,b in MECH: s=re.sub(a,b,s)
    return s
def norm_path_ours(p):  return p.replace("rag3db","kuzu")
def norm_path_theirs(p):return p.replace("ladybug","kuzu").replace("lbug","kuzu")

rc,out,_=git("diff","--name-only",UP,NS,"--","src/")
cands=[p for p in out.splitlines() if p.strip()]

really=[]; rename_only=0
for p in cands:
    a=show(UP,norm_path_ours(p)) 
    if a is None: a=show(UP,p)
    b=show(NS,p)
    if b is None: continue
    if a is None:
        really.append((p,"neuf")); continue
    if norm_ours(b)==a: rename_only+=1
    else: really.append((p,"greffon"))

print(f"src/ touches (brut)          : {len(cands)}")
print(f"  purement le renommage      : {rename_only}")
print(f"  reellement a nous          : {len(really)}")
neufs=[p for p,k in really if k=="neuf"]
shared=[p for p,k in really if k=="greffon"]
print(f"    dont entierement a nous  : {len(neufs)}")
print(f"    dont greffons partages   : {len(shared)}")
print()
for p in neufs: print("   NEUF   ",p)
print()

for mode in ("strict","dialecte"):
    d=os.path.join(SB,mode)
    if os.path.exists(d): shutil.rmtree(d)
    os.makedirs(d)
    clean=[];conf=[];absent=[]
    for p in shared:
        pu=norm_path_ours(p)
        up=show(UP,pu) or show(UP,p)
        ns=show(NS,p)
        lb=show(LB,pu) or show(LB,norm_path_theirs(pu))
        if lb is None: absent.append(p); continue
        up_n=up; ns_n=norm_ours(ns); lb_n=norm_theirs(lb)
        if mode=="dialecte": up_n=dialect(up_n); ns_n=dialect(ns_n)
        paths={}
        for tag,c in (("up",up_n),("ns",ns_n),("lb",lb_n)):
            fp=os.path.join(d,tag,pu); os.makedirs(os.path.dirname(fp),exist_ok=True)
            open(fp,"w").write(c); paths[tag]=fp
        r=subprocess.run(["git","merge-file","-p","--diff3",paths["lb"],paths["up"],paths["ns"]],
                         capture_output=True,text=True)
        open(os.path.join(d,"m_"+pu.replace("/","_")),"w").write(r.stdout)
        if r.returncode==0: clean.append(p)
        else: conf.append((p,r.returncode))
    print(f"=== reapplication de nos greffons sur leur arbre — mode {mode} ===")
    print(f"  se reposent proprement : {len(clean)} / {len(shared)}")
    print(f"  en conflit             : {len(conf)}")
    print(f"  absents chez eux       : {len(absent)}")
    if conf:
        print("  conflits — fichier : nombre de zones a rearbitrer")
        for p,n in sorted(conf,key=lambda x:-x[1]): print(f"    {n:>2}  {p}")
    if absent: print("  absents :",absent)
    print()
