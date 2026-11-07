#!/bin/bash
# **Les hooks de Claude Code de la mémoire longue** (page docs/07-novembre-2026-hooks-claude-code/01).
#
# Pose le réglage (~/.config/rag3weaver/hooks.json) et, SUR DEMANDE EXPLICITE (`--poser`), les
# cinq entrées de hooks dans ~/.claude/settings.json : SessionStart, UserPromptSubmit, PreCompact,
# PostCompact, Stop, chacune lançant `<binaire> hook`. Sans `--poser`, le script montre ce qu'il
# poserait et ne touche à rien : settings.json est à Lucie.
#
#   scripts/hooks_claude_code.sh [--binaire <chemin>] [--settings <chemin>] [--config <chemin>]
#                                 [--manifest <backend.json>] [--poser] [--retirer] [--montrer]
#
# - Idempotent : relancé, il ne change rien et le dit. Les entrées sont reconnues à leur commande
#   (`<binaire> hook`) ; les autres hooks de settings.json ne sont pas touchés.
# - `--retirer` enlève les siennes et rien d'autre ; le réglage reste.
# - Les délais par entrée : ceux du réglage (5 s / 20 s) plus cinq secondes, en secondes entières.
# - Le fichier d'avant est gardé à côté (`settings.json.avant-hooks`) à la première pose.
set -euo pipefail

binaire="$HOME/.local/bin/rag3weaver-backend"
settings="$HOME/.claude/settings.json"
config="$HOME/.config/rag3weaver/hooks.json"
manifest="$HOME/.config/rag3weaver/memoire-longue/backends/memory/backend.json"
poser=0; retirer=0; montrer=0
while [ $# -gt 0 ]; do
  case "$1" in
    --binaire) binaire="$2"; shift 2 ;;
    --settings) settings="$2"; shift 2 ;;
    --config) config="$2"; shift 2 ;;
    --manifest) manifest="$2"; shift 2 ;;
    --poser) poser=1; shift ;;
    --retirer) retirer=1; shift ;;
    --montrer) montrer=1; shift ;;
    *) echo "option inconnue : $1" >&2; sed -n 2,16p "$0" >&2; exit 2 ;;
  esac
done

commande="$binaire hook --config $config"

# Le réglage : les défauts, le manifeste donné ; jamais réécrit s'il existe.
if [ "$retirer" = 0 ]; then
  if [ -f "$config" ]; then
    echo "réglage déjà là : $config (inchangé)"
  elif [ "$poser" = 1 ] || [ "$montrer" = 1 ]; then
    reglage=$(python3 - "$manifest" "$HOME" <<'EOF'
import json, sys
m, home = sys.argv[1], sys.argv[2]
print(json.dumps({
  "manifest": m, "keys": ["dev"], "context_tokens": 1000000, "suggest_ratio": 0.7,
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

# Les entrées, fusionnées dans settings.json par python (le JSON de settings est à préserver tel quel).
python3 - "$settings" "$commande" "$poser" "$retirer" "$montrer" <<'EOF'
import json, os, sys, shutil
settings, commande, poser, retirer, montrer = sys.argv[1], sys.argv[2], sys.argv[3] == "1", sys.argv[4] == "1", sys.argv[5] == "1"
entrees = {
  "SessionStart":     ("startup|resume|clear|compact", 25),
  "UserPromptSubmit": (None, 10),
  "PreCompact":       ("manual|auto", 25),
  "PostCompact":      ("manual|auto", 25),
  "Stop":             (None, 25),
}
def est_a_nous(h):
    return any(x.get("type") == "command" and str(x.get("command", "")).startswith(commande.split(" --config")[0]) for x in h.get("hooks", []))
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
        entree = {"hooks": [{"type": "command", "command": commande, "timeout": delai}]}
        if matcher: entree["matcher"] = matcher
        liste.append(entree)
    if liste: hooks[ev] = liste
    elif ev in hooks: del hooks[ev]
if not hooks: del d["hooks"]
apres = json.dumps(d, sort_keys=True)
if apres == avant:
    print("settings.json : rien à changer (%s)" % settings); sys.exit(0)
if poser:
    garde = settings + ".avant-hooks"
    if os.path.exists(settings) and not os.path.exists(garde):
        shutil.copy2(settings, garde); print("l'ancien settings.json gardé : " + garde)
    os.makedirs(os.path.dirname(settings) or ".", exist_ok=True)
    tmp = settings + ".tmp"
    with open(tmp, "w", encoding="utf-8") as f:
        json.dump(d, f, indent=2, ensure_ascii=False); f.write("\n")
    os.replace(tmp, settings)
    print(("hooks retirés de " if retirer else "cinq hooks posés dans ") + settings)
else:
    print("settings.json changerait ainsi (relancer avec --poser pour le faire) :")
    print(json.dumps(d.get("hooks", {}), indent=2, ensure_ascii=False))
EOF
