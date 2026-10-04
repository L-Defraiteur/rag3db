#!/bin/bash
# Joue une commande et note le pic de mémoire de sa portée (cgroup v2), à lancer sous `poste`.
out="$1"; shift
"$@" > "$out" 2>&1
code=$?
cg=/sys/fs/cgroup$(cut -d: -f3 /proc/self/cgroup)
echo "code $code ; pic $(( $(cat "$cg/memory.peak" 2>/dev/null || echo 0) / 1048576 )) Mio ; portée $cg" > "$out.pic"
exit $code
