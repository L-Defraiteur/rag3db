# Rouge d'entrée de la qualification ties, avant Cargo

Ticket créé avant correctif/transmission :
[cache privé et magasin de libs](../../tickets/closed/2026-11-16-le-pilote-ties-isole-le-cache-sans-epingler-le-magasin-de-libs.md).

Sourceproduitbd76/pilote6d1e ont été joués sous l'allocation fraîche C7.
La garde du target neuf est passée ; le premier `run_e2e.sh --lib` s'arrête
code2 car le driver isole XDG_CACHE_HOME sans fixer RAG3DB_LIBS_DIR. La
préparation avait correctement vérifié les binaires du magasin réel, mais
le profil de son enfant n'en transportait pas le chemin. Aucun juge, ELF ou
F8 n'est acquis pour ce candidat ; aucun Cargo lancé d'après ce refus
préliminaire et l'absence d'empreintes du target.

Le premierrouge est conservé, pas remplacé par une lecture ultérieure.
ScopeROUGE/code1/restes[]/nettoyétrue ; reçu terminé ; pas de bus conservé.
La correction de profil n'est pas encore appliquée dans ce commit. Le
prochain pilote doit être publié et relu avant entrée, avec P/Q/S neufs et
garde du target même source ; borne de reprise à arbitrer C7, pas de reset
automatique7200. Aucun push master, service ou base de Lucie modifié.
