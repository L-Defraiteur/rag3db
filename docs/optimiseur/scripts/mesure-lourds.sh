#!/bin/bash
# Mesure de la charge par lourd (14 nov. 2026, demande de c7) : toutes les 10 s pendant <durée> s, pour chaque portée
# systemd-run --user (run-*.scope, ce que `poste lourd` crée), le CPU consommé sur l'intervalle (en % d'un cœur, par
# cpu.stat usage_usec), le nombre de tâches (pids.current), la mémoire (memory.current), et sa commande ; plus la
# charge du poste. Lecture seule. Usage : mesure-lourds.sh <durée en s> > journal.tsv
set -u
DUREE=${1:-600}; PAS=10
U=/sys/fs/cgroup/user.slice/user-$(id -u).slice/user@$(id -u).service/app.slice
[ -d "$U" ] || U=/sys/fs/cgroup/user.slice/user-$(id -u).slice/user@$(id -u).service
declare -A avant
printf 'heure\tcharge1\tportee\tcpu_pct\ttaches\tmem_mio\tcommande\n'
fin=$(( $(date +%s) + DUREE ))
while [ "$(date +%s)" -lt "$fin" ]; do
  h=$(date +%T); c=$(cut -d' ' -f1 /proc/loadavg)
  for d in $(find /sys/fs/cgroup/user.slice/user-$(id -u).slice -maxdepth 4 -type d -name 'run-*.scope' 2>/dev/null); do
    n=$(basename "$d"); u=$(awk '/^usage_usec/{print $2}' "$d/cpu.stat" 2>/dev/null) || continue
    if [ -n "${avant[$n]:-}" ]; then
      pct=$(( (u - ${avant[$n]}) / (PAS * 10000) ))
      t=$(cat "$d/pids.current" 2>/dev/null); m=$(( $(cat "$d/memory.current" 2>/dev/null || echo 0) / 1048576 ))
      cmd=$(systemctl --user show "$n" -p Description --value 2>/dev/null | sed 's/^\[systemd-run\] //' | cut -c1-110)
      printf '%s\t%s\t%s\t%s\t%s\t%s\t%s\n' "$h" "$c" "$n" "$pct" "$t" "$m" "$cmd"
    fi
    avant[$n]=$u
  done
  sleep $PAS
done
