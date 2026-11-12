# granite-278m à luciepc s'est arrêté huit fois à 10 h 47 (code 1 après 7 s, 4,2 Go) ; cause illisible, le journal avait tourné

- **État** : ouvert (8 novembre 2026, session embarquement)
- **Famille** : produit
- **Gravité** : panne de service — 7878 injoignable de 10 h 47 à 12 h 07 ; le réveil de l'hôte de
  luciepc l'a refusé à 12 h 06 (un backend ouvert sans son embarqueur le garde absent), code-principal
  et code-lourd hors service, la réindexation de G retardée
- **Atteignable en service** : oui, l'unité `rag3weaver-embeddings@granite-278m` de luciepc
- **Touche rag3weaver** : le service d'embarquement (`rag3weaver-embeddings`), ou la carte ; on ne
  sait pas lequel

## Ce que c'est

`systemctl --user status rag3weaver-embeddings@granite-278m` à 12 h 06 : `failed (Result:
start-limit-hit) since Sun 2026-11-08 10:47:54 CET`, dernier départ à 10 h 47 m 42 s, sortie en
`status=1/FAILURE` à 10 h 47 m 49 s (7 s, pic de mémoire 4,2 Go), `NRestarts=8`. Le plafond de
l'unité (`StartLimitBurst=3` en 5 min, posé le 25 octobre) l'a arrêtée au troisième départ de la
rafale ; rien ne l'a relancée ensuite.

7 s et 4,2 Go, c'est le temps d'un chargement de granite-278m et du début de sa chauffe (au départ
de 12 h 07 : chargé en 2,1 s, chauffe de 3,5 s). Le processus est donc allé jusqu'au modèle avant
de sortir en erreur.

## Ce qui était su, et ce qui ne l'est plus

- **Illisible** : le journal `--user` de luciepc ne commence qu'à 11 h 17 m 55 s (« No entries »
  pour l'unité), le journal du noyau n'a rien entre 10 h 40 et 10 h 50, et `dmesg` demande sudo. Ni
  le message d'erreur du service, ni un éventuel reset de l'anneau gfx de card2, ni le processus qui
  aurait tenu la carte à 10 h 47 ne se lisent.
- **Lu à 12 h 06-12 h 07, avant la relance** : rien n'écoutait sur 7878 ; mémoire vive disponible
  70 Go sur 93 ; card2 (04:00.0) à 12,1 Go sur 34, donc 22 Go libres ; card0 à 0,09 Go ; 7879
  (bge-m3, 1024) et 7880 (granite-107m, 384) répondaient, leurs unités n'avaient pas bougé.
- **Le retour** : un seul départ contrôlé (`reset-failed` puis `start`) à 12 h 07 m 24 s, à
  l'écoute à 12 h 07 m 30 s, `NRestarts=0`, dense à écart absolu nul sur le vecteur témoin noté le
  matin, card2 de 12,1 à 16,1 Go.

## La mesure qui manquait

Le journal du service lui-même, gardé hors de la rotation du système. Posé le 8 novembre à 12 h 08
pour les trois unités, par une surcharge `~/.config/systemd/user/rag3weaver-embeddings@.service.d/journal.conf` :
`StandardOutput` et `StandardError` en `append` dans
`~/.cache/rag3weaver-unites-embarquement/journal-<modèle>.log`. Elle prend effet au prochain départ
de chaque unité : posée après le départ de 12 h 07, elle ne vaut pour aucune des trois avant leur
prochain redémarrage.

## Ce qu'il faut pour le fermer

Relevé complémentaire pendant l'audit du 12 novembre : service actuellement
disponible, `NRestarts=4`, départ indiqué le 11 novembre à 23:20:40 ; journal
du service désormais présent (3 263 octets, 34 lignes). Filtrage de métadonnées
par embarquement : quatre mentions de chargement/chauffe/écoute et deux mentions
Error, sans occurrence panicked/OOM/DeviceLost/timeout. Ces nombres ne prouvent
ni une nouvelle panne spontanée ni sa cause : opérations et journal restent
à corréler. Aucun redémarrage de ce service demandé par l'audit ; ticket ouvert.

À la prochaine sortie d'une de ces unités, lire son `journal-<modèle>.log` et nommer la cause
(une erreur du service, la carte, la mémoire) ; puis la corriger, ou fermer ce ticket comme un
accident nommé. Sans nouvelle sortie dans le mois, le fermer en le disant.
