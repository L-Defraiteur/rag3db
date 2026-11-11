#!/bin/bash
# Lecture de charge à côté de toute preuve ou mesure à luciepc (c7, 8 nov. 2026) : Lucie joue parfois sur seat1.
echo "== lecture $(date +%T) : $(uptime | sed 's/.*load/load/')"
rocm-smi --showbus --showuse --showmemuse 2>&1 | grep -E 'PCI Bus|GPU use|VRAM%' | sed 's/^/   /'
echo "   seat1 : $(loginctl seat-status seat1 2>/dev/null | grep -o 'drm:card[0-9]' | head -1) ; sessions seat1 : $(loginctl list-sessions --no-legend | awk '$4=="seat1"' | wc -l)"
