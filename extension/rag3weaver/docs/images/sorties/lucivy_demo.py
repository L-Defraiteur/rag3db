import os, shutil, sys, lucivy
# The rag3weaver sources, indexed and searched by lucivy alone (no database, no rag3weaver).
root = sys.argv[1]
path = os.path.join(os.environ.get("TMPDIR", "/tmp"), "lucivy-demo-index")
shutil.rmtree(path, ignore_errors=True)
index = lucivy.Index.create(path, [
    {"name": "path", "type": "text", "stored": True},
    {"name": "body", "type": "text", "stored": True},
])
paths = []
n = 0
for d, _, files in os.walk(os.path.join(root, "src")):
    for f in files:
        if f.endswith(".rs"):
            p = os.path.join(d, f)
            index.add(n, path=os.path.relpath(p, root), body=open(p, encoding="utf-8", errors="replace").read())
            paths.append(os.path.relpath(p, root))
            n += 1
index.commit()
print(f"indexed {n} Rust files")
print('search "worktree", 5 best:')
for r in index.search("worktree", limit=5):
    print(f"  {r.score:7.4f}  {paths[r.doc_id]}")
