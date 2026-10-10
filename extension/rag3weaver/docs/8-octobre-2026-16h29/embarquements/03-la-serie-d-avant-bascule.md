# La série d'avant bascule — 10 octobre 2026

Session embarquements, chantier F.

**Ce que mesure cette série :**
- Lib du moteur de 10:46 (`fb98852e1` + `71cffbc4b`). La **fuite de pages
  au rejeu n'y est pas corrigée** : cette série ne vaut pas pour l'accord du
  basculement.
- Toutes les passes ont déjà la forme du nouveau défaut de rag3weaver :
  paquets de 2 048 fichiers, transaction par paquet, une validation par
  paquet (K = 1).
- Ce qui varie, c'est le moteur : **COPY forcé** (le défaut du moteur, avec
  le saut du journal en mémoire de `fb98852e1`) ou **COPY journalisé**
  (`RAG3WEAVER_ESTIMATE_COPY_JOURNALISE=1`).
- Dépôt entier, base sur disque, `poste mesure`.

## Les passes

Le calme est atteint quand la pression d'E/S (avg10) passe sous 2 %. Une
passe partie sans calme est marquée **✗** et n'entre pas dans les médianes.

| Passe | Mode | COPY | Durée | Pic | Attente du calme | Pression au départ |
|---|---|---|---|---|---|---|
| 1 | fichiers | forcé | 96 s ✗ | 9,2 Go | 181 s (bornée) | 13,6 % |
| 1 | fichiers | journalisé | 97 s ✗ | 9,4 Go | 181 s (bornée) | 12,2 % |
| 1 | blobs | forcé | 119 s ✗ | 14,0 Go | 181 s (bornée) | 8,2 % |
| 1 | blobs | journalisé | 125 s ✗ | 13,8 Go | 181 s (bornée) | 13,0 % |
| 2 | fichiers | forcé | 91 s ✗ | 9,2 Go | 180 s (bornée) | 18,4 % |
| 2 | fichiers | journalisé | 92 s ✗ | 9,5 Go | 181 s (bornée) | 12,0 % |
| 2 | blobs | forcé | 116 s | 13,6 Go | 22 s | 1,8 % |
| 2 | blobs | journalisé | 119 s ✗ | 14,5 Go | 180 s (bornée) | 15,4 % |
| 3 | fichiers | forcé | 78 s | 9,3 Go | 98 s | 1,7 % |
| 3 | fichiers | journalisé | 81 s | 9,2 Go | 0 s | 0,4 % |
| 3 | blobs | forcé | 93 s | 14,0 Go | 0 s | 1,9 % |
| 3 | blobs | journalisé | 98 s | 14,2 Go | 0 s | 0,4 % |
| 4 | fichiers | forcé | 78 s | 9,2 Go | 0 s | 1,0 % |
| 4 | fichiers | journalisé | 86 s | 9,4 Go | 0 s | 0,2 % |
| 4 | blobs | forcé | 89 s | 14,0 Go | 0 s | 0,6 % |
| 4 | blobs | journalisé | 91 s | 14,6 Go | 22 s | 1,7 % |
| 5 | fichiers | forcé | 83 s | 9,0 Go | 12 s | 2,0 % |
| 5 | fichiers | journalisé | 82 s | 9,2 Go | 0 s | 0,6 % |
| 5 | blobs | forcé | 104 s | 13,9 Go | 0 s | 0,1 % |
| 5 | blobs | journalisé | — | — | — | arrêtée pour le redémarrage du poste |

Les tours 4 et 5 reprennent les tours 1 et 2, gâtés : une autre activité
tenait l'E/S entre 8 et 18 % pendant la tenue exclusive. Toutes les passes
sont vertes, 7 031 fichiers et 255 899 relations chacune, et
`copy_journal_fallbacks` vaut 0 partout. Sorties :
`~/.cache/rag3weaver-build/mesure-ab-*.out`, `serie-avant-bascule{,-2}.out`.

## Ce que ça dit (passes calmes seulement)

| Mode | COPY forcé | COPY journalisé | Écart |
|---|---|---|---|
| fichiers | 78, 78, 83 → **78 s** | 81, 86, 82 → **82 s** | **+5 %**, hors des ±3 % exigés |
| blobs | 116, 93, 89, 104 → **≈ 98 s** (dispersé) | 98, 91 → deux passes seulement | pas de conclusion |

- **Le COPY journalisé, en mode fichiers, coûte environ 4 s** (de 78 à
  82 s), sur un moteur où la fuite n'est pas corrigée. Le journal y pèse
  166 Mo au plus, et 489 Mo au total. Avant l'accord du basculement, la
  série se rejoue sur la lib corrigée.
- **Ce que `fb98852e1` a fait gagner au COPY forcé**, contre la référence
  du 5 octobre (moteur `eb2d78e46`, même forme 2 048 × 1) :
  - temps : rien de mesurable, ni en fichiers (78 s, comme le 5 octobre) ni
    en blobs (89–104 s ici, 78–89 s le 5 octobre, plus dispersé) ;
  - mémoire : un peu en blobs, avec des pics de 13,6–14,0 Go contre
    14,2–14,7 Go (≈ −0,5 Go, à rapprocher des 849 Mio que le moteur ne
    sérialise plus) ; rien en fichiers (9,0–9,3 Go contre 9,1–9,4 Go).
- **La première chose cherchable** arrive entre 8 et 15 s, comme le 5
  octobre.

## À rejouer

Sur la lib qui portera le correctif de la fuite : les mêmes douze passes,
dans une seule tenue, avec la pression et l'attente par passe. Il faudra
aussi la passe manquante (blobs, COPY journalisé, 5ᵉ tour), plus d'une
passe en blobs pour lisser leur dispersion, et le nouveau défaut écrit en
tête.
