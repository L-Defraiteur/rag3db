#!/bin/bash
# **Les hooks de Claude Code de la mémoire longue** (page docs/07-novembre-2026-hooks-claude-code/01).
#
# Pose le réglage (~/.config/rag3weaver/hooks.json) et, SUR DEMANDE EXPLICITE (`--poser`), les
# six entrées de hooks (SessionStart, UserPromptSubmit, PreCompact, PostCompact, Stop, PostToolUse), chacune
# lançant `<binaire> hook`, dans UNE cible — trois niveaux, du plus étroit au plus large :
#
#   (défaut)              le `.claude/settings.local.json` du dossier courant : une seule session,
#                         celle qu'on lance dans ce dossier ;
#   --settings <fichier>  un fichier à soi, à passer à `claude --settings <fichier>` ;
#   --global              le `~/.claude/settings.json` de Lucie — jamais sans son mot.
#
#   --worktree <nom>      l'essai sur du vrai travail : `git worktree add ~/git_workspaces/rag3db-<nom>
#                         -b <nom>` depuis master, le sous-module à jour, les hooks dans son
#                         `.claude/settings.local.json`, et la commande à lancer imprimée.
#
#   scripts/hooks_claude_code.sh [--binaire <chemin>] [--config <chemin>] [--manifest <backend.json>]
#                                 [--settings <fichier> | --global | --worktree <nom>]
#                                 [--poser] [--retirer] [--montrer]
#
# - Sans `--poser`, le script montre ce qu'il poserait et ne touche à rien.
# - Idempotent : relancé, il ne change rien et le dit. Les entrées sont reconnues à leur commande
#   (`<binaire> hook`) ; les autres hooks du fichier ne sont pas touchés.
# - `--retirer` enlève les siennes et rien d'autre ; le réglage reste.
# - Les délais par entrée : ceux du réglage (5 s / 20 s) plus cinq secondes, en secondes entières.
# - Le fichier d'avant est gardé à côté (`<fichier>.avant-hooks`) à la première pose.
set -euo pipefail

binaire="$HOME/.local/bin/rag3weaver-backend"
settings=""
config="$HOME/.config/rag3weaver/hooks.json"
manifest="$HOME/.config/rag3weaver/memoire-longue/backends/memory/backend.json"
worktree=""
global=0; poser=0; retirer=0; montrer=0
while [ $# -gt 0 ]; do
  case "$1" in
    --binaire) binaire="$2"; shift 2 ;;
    --settings) settings="$2"; shift 2 ;;
    --global) global=1; shift ;;
    --worktree) worktree="$2"; shift 2 ;;
    --config) config="$2"; shift 2 ;;
    --manifest) manifest="$2"; shift 2 ;;
    --poser) poser=1; shift ;;
    --retirer) retirer=1; shift ;;
    --montrer) montrer=1; shift ;;
    *) echo "option inconnue : $1" >&2; sed -n 2,27p "$0" >&2; exit 2 ;;
  esac
done
if [ "$global" = 1 ] && [ -n "$settings$worktree" ]; then echo "--global s'exclut avec --settings et --worktree" >&2; exit 2; fi
if [ -n "$settings" ] && [ -n "$worktree" ]; then echo "--settings et --worktree s'excluent" >&2; exit 2; fi

# La cible.
if [ "$global" = 1 ]; then
  settings="$HOME/.claude/settings.json"; niveau="global (le settings.json de Lucie)"
elif [ -n "$worktree" ]; then
  dossier="${RAG3WEAVER_WORKTREES:-$HOME/git_workspaces}/rag3db-$worktree"
  depot="${RAG3WEAVER_DEPOT:-$HOME/git_workspaces/rag3db}"
  if [ "$poser" = 1 ] && [ "$retirer" = 0 ]; then
    if [ -d "$dossier" ]; then
      echo "worktree déjà là : $dossier (inchangé)"
    else
      git -C "$depot" fetch -q origin master
      git -C "$depot" worktree add -q "$dossier" -b "$worktree" origin/master
      git -C "$dossier" submodule update -q --init extension/rag3weaver/codeparsers 2>/dev/null || true
      echo "worktree créé : $dossier (branche $worktree, depuis origin/master)"
    fi
  fi
  settings="$dossier/.claude/settings.local.json"; niveau="le worktree $dossier"
elif [ -n "$settings" ]; then
  niveau="le fichier $settings (à passer à : claude --settings $settings)"
else
  settings="$PWD/.claude/settings.local.json"; niveau="le dossier courant ($PWD)"
fi
echo "cible : $niveau"

# La commande posée dans settings est stable et ne change jamais : le binaire, `hook`, l'événement.
# Tout le reste (hooks.json, les gabarits, le binaire lui-même) se relit à chaque appel.
commande="$binaire hook"

# Le réglage : les défauts, le manifeste donné ; jamais réécrit s'il existe.
if [ "$retirer" = 0 ]; then
  if [ -f "$config" ]; then
    echo "réglage déjà là : $config (inchangé)"
  elif [ "$poser" = 1 ] || [ "$montrer" = 1 ]; then
    reglage=$(python3 - "$manifest" "$HOME" <<'EOF'
import json, sys
m, home = sys.argv[1], sys.argv[2]
print(json.dumps({
  "manifest": m, "keys": ["dev"], "context_tokens": 1000000, "suggest_ratio": 0.7, "templates_dir": home + "/.config/rag3weaver/hooks",
  "prompt_block_chars": 1500, "session_block_chars": 3000,
  "prompt_timeout_ms": 5000, "timeout_ms": 20000,
  "journal": home + "/.local/state/rag3weaver/hooks.log",
}, indent=2, ensure_ascii=False))
EOF
)
    if [ "$poser" = 1 ]; then
      mkdir -p "$(dirname "$config")"; printf '%s\n' "$reglage" > "$config"; echo "réglage posé : $config"
    else
      echo "réglage qui serait posé dans $config :"; printf '%s\n' "$reglage"
    fi
  fi
fi

# Les gabarits de blocs, posés par le binaire s'il est là (jamais écrasés) : c'est là qu'on corrige à la volée.
if [ "$retirer" = 0 ] && [ "$poser" = 1 ] && [ -x "$binaire" ]; then
  "$binaire" hook --poser-les-gabarits --config "$config" || echo "gabarits : non posés ($binaire hook --poser-les-gabarits a échoué)"
fi

# Les entrées, fusionnées dans la cible par python (le JSON du fichier est à préserver tel quel).
python3 - "$settings" "$commande" "$poser" "$retirer" <<'EOF'
import json, os, sys, shutil
settings, commande, poser, retirer = sys.argv[1], sys.argv[2], sys.argv[3] == "1", sys.argv[4] == "1"
entrees = {
  "SessionStart":     ("startup|resume|clear|compact", 25),
  "UserPromptSubmit": (None, 10),
  "PreCompact":       ("manual|auto", 25),
  "PostCompact":      ("manual|auto", 25),
  "Stop":             (None, 25),
  # Le rappel différé (3 ter b) : lu à chaque outil, sans hôte ; délai court.
  "PostToolUse":      (None, 5),
}
def est_a_nous(h):
    return any(x.get("type") == "command" and str(x.get("command", "")).startswith(commande) for x in h.get("hooks", []))
try:
    with open(settings, encoding="utf-8") as f:
        d = json.load(f)
except FileNotFoundError:
    d = {}
hooks = d.setdefault("hooks", {})
avant = json.dumps(d, sort_keys=True)
for ev, (matcher, delai) in entrees.items():
    liste = [h for h in hooks.get(ev, []) if not est_a_nous(h)]
    if not retirer:
        entree = {"hooks": [{"type": "command", "command": commande + " " + ev, "timeout": delai}]}
        if matcher: entree["matcher"] = matcher
        liste.append(entree)
    if liste: hooks[ev] = liste
    elif ev in hooks: del hooks[ev]
if not hooks: del d["hooks"]
apres = json.dumps(d, sort_keys=True)
if apres == avant:
    print("rien à changer dans %s" % settings); sys.exit(0)
anciennes = sorted({x.get("command") for ev in entrees for h in json.loads(avant).get("hooks", {}).get(ev, []) if est_a_nous(h) for x in h.get("hooks", []) if not str(x.get("command", "")).endswith(" " + ev)})
if anciennes and not retirer:
    print("commande d'avant remplacée par la commande stable « %s <Événement> » : %s" % (commande, ", ".join(anciennes)))
if poser:
    garde = settings + ".avant-hooks"
    if os.path.exists(settings) and not os.path.exists(garde):
        shutil.copy2(settings, garde); print("l'ancien fichier gardé : " + garde)
    os.makedirs(os.path.dirname(settings) or ".", exist_ok=True)
    tmp = settings + ".tmp"
    with open(tmp, "w", encoding="utf-8") as f:
        json.dump(d, f, indent=2, ensure_ascii=False); f.write("\n")
    os.replace(tmp, settings)
    print(("hooks retirés de " if retirer else "cinq hooks posés dans ") + settings)
else:
    print("le fichier changerait ainsi (relancer avec --poser pour le faire) :")
    print(json.dumps(d.get("hooks", {}), indent=2, ensure_ascii=False))
EOF

if [ -n "$worktree" ] && [ "$poser" = 1 ] && [ "$retirer" = 0 ]; then
  echo
  echo "l'essai : cd $dossier && claude"
  echo "  (le bloc [mémoire] apparaît au premier tour ; le journal : ~/.local/state/rag3weaver/hooks.log)"
  echo "corriger à la volée : les gabarits dans $(dirname "$config")/hooks/<Événement>.md (une section ## par bloc), relus à chaque appel ;"
  echo "  vérifier hors session : $binaire hook --rejouer <transcription.jsonl> SessionStart"
  echo "retirer : $0 --worktree $worktree --retirer --poser"
fi
