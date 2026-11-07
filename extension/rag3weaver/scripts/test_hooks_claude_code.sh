#!/bin/bash
# Le témoin de scripts/hooks_claude_code.sh : sur un fichier d'essai qui porte déjà un autre hook —
# rien sans --poser ; cinq entrées posées ; relancé, rien ne change ; --retirer rend l'original ;
# la cible par défaut est le .claude/settings.local.json du dossier courant ; --worktree crée un
# worktree d'un dépôt d'essai et y pose les hooks ; --global s'exclut des autres cibles.
set -euo pipefail
ICI="$(cd "$(dirname "$0")" && pwd)"
T=$(mktemp -d); trap 'rm -rf "$T"' EXIT
S="$T/settings.json"; C="$T/hooks.json"
cat > "$S" <<'EOF'
{"model": "opus", "hooks": {"Stop": [{"hooks": [{"type": "command", "command": "/usr/bin/autre --chose"}]}]}}
EOF
cp "$S" "$T/original.json"
rouge() { echo "ROUGE : $1" >&2; exit 1; }
sh="$ICI/hooks_claude_code.sh"
args=(--binaire /opt/r/rag3weaver-backend --settings "$S" --config "$C" --manifest /m/backend.json)

"$sh" "${args[@]}" --montrer > "$T/montre.out"
cmp -s "$S" "$T/original.json" || rouge "--montrer a touché le fichier"
[ -f "$C" ] && rouge "--montrer a posé le réglage"
grep -q '"SessionStart"' "$T/montre.out" || rouge "--montrer ne montre pas les entrées"

"$sh" "${args[@]}" --poser > "$T/pose.out"
[ -f "$C" ] || rouge "le réglage n'est pas posé"
python3 - "$S" "$C" <<'EOF' || rouge "les entrées posées ne sont pas celles attendues"
import json, sys
d = json.load(open(sys.argv[1])); h = d["hooks"]; c = json.load(open(sys.argv[2]))
assert d["model"] == "opus", "le reste du fichier est préservé"
assert c["manifest"] == "/m/backend.json" and c["timeout_ms"] == 20000
for ev in ["SessionStart", "UserPromptSubmit", "PreCompact", "PostCompact", "Stop"]:
    notres = [x for x in h[ev] if x["hooks"][0]["command"].startswith("/opt/r/rag3weaver-backend hook")]
    assert len(notres) == 1, ev
    assert notres[0]["hooks"][0]["command"] == "/opt/r/rag3weaver-backend hook " + ev, "la commande est stable : binaire, hook, événement — " + notres[0]["hooks"][0]["command"]
assert h["SessionStart"][0]["matcher"] == "startup|resume|clear|compact"
assert "matcher" not in h["UserPromptSubmit"][0] and "matcher" not in h["Stop"][-1]
assert h["UserPromptSubmit"][0]["hooks"][0]["timeout"] == 10
autres = [x for x in h["Stop"] if x["hooks"][0]["command"] == "/usr/bin/autre --chose"]
assert len(autres) == 1, "l'autre hook de Stop est gardé"
EOF
[ -f "$S.avant-hooks" ] || rouge "l'ancien fichier n'est pas gardé"
cp "$S" "$T/pose1.json"
"$sh" "${args[@]}" --poser > "$T/pose2.out"
cmp -s "$S" "$T/pose1.json" || rouge "une seconde pose a changé le fichier"
grep -q 'rien à changer' "$T/pose2.out" || rouge "la seconde pose ne dit pas qu'elle ne change rien"

"$sh" "${args[@]}" --retirer --poser > "$T/retire.out"
python3 - "$S" "$T/original.json" <<'EOF' || rouge "--retirer ne rend pas l'original"
import json, sys
assert json.load(open(sys.argv[1])) == json.load(open(sys.argv[2]))
EOF

# La cible par défaut : le dossier courant.
mkdir -p "$T/dossier"
( cd "$T/dossier" && "$sh" --binaire /opt/r/rag3weaver-backend --config "$C" --poser > "$T/local.out" )
[ -f "$T/dossier/.claude/settings.local.json" ] || rouge "la cible par défaut n'est pas .claude/settings.local.json du dossier courant"
grep -q 'cible : le dossier courant' "$T/local.out" || rouge "la cible par défaut n'est pas dite"
[ -f "$HOME/.claude/settings.json.avant-hooks" ] && [ "$HOME/.claude/settings.json.avant-hooks" -nt "$T/original.json" ] && rouge "le settings.json de Lucie a été touché"

# --worktree sur un dépôt d'essai.
git init -q "$T/depot" && git -C "$T/depot" -c user.email=t@t -c user.name=t commit -q --allow-empty -m init && git -C "$T/depot" branch -M master
git init -q --bare "$T/origin.git" && git -C "$T/depot" remote add origin "$T/origin.git" && git -C "$T/depot" push -q origin master
mkdir -p "$T/wts"
RAG3WEAVER_WORKTREES="$T/wts" RAG3WEAVER_DEPOT="$T/depot" "$sh" --binaire /opt/r/rag3weaver-backend --config "$C" --worktree essai --poser > "$T/wt.out"
[ -d "$T/wts/rag3db-essai/.git" ] || [ -f "$T/wts/rag3db-essai/.git" ] || rouge "le worktree n'est pas créé"
[ "$(git -C "$T/wts/rag3db-essai" rev-parse --abbrev-ref HEAD)" = essai ] || rouge "le worktree n'est pas sur la branche essai"
[ -f "$T/wts/rag3db-essai/.claude/settings.local.json" ] || rouge "les hooks ne sont pas dans le settings.local.json du worktree"
grep -q "cd $T/wts/rag3db-essai && claude" "$T/wt.out" || rouge "la commande à lancer n'est pas imprimée"
RAG3WEAVER_WORKTREES="$T/wts" RAG3WEAVER_DEPOT="$T/depot" "$sh" --binaire /opt/r/rag3weaver-backend --config "$C" --worktree essai --poser > "$T/wt2.out"
grep -q 'worktree déjà là' "$T/wt2.out" && grep -q 'rien à changer' "$T/wt2.out" || rouge "une seconde pose du worktree n'est pas idempotente"

# --global s'exclut.
if "$sh" --global --settings "$S" --montrer > /dev/null 2>&1; then rouge "--global et --settings devraient s'exclure"; fi
echo "VERT : hooks_claude_code.sh — montrer sans toucher, poser, idempotent, retirer, dossier courant, worktree, exclusions"
