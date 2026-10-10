# La série de confirmation du basculement — 10 octobre 2026

Session embarquements, chantier F. Elle fait suite à
[03-la-serie-d-avant-bascule.md](03-la-serie-d-avant-bascule.md), jouée sur
une lib où la fuite n'était pas corrigée.

**Ce que mesure cette série :**
- **Code et moteur** : master `21a1d67c5`, avec les nouveaux défauts de
  rag3weaver. La lib commune et l'extension vector sont rebâties à 15:36 sur
  le moteur `1275099c2` : la fuite de pages est corrigée (`0aed3c4b5`,
  version de stockage 40).
- **Forme de toutes les passes** : le nouveau défaut. Paquets de 2 048
  fichiers, transaction par paquet, une validation par paquet.
- **Ce qui varie** :
  - le plein texte : en fichiers (le défaut d'une base neuve) ou en base,
    forcé par `RAG3WEAVER_FTS=blobs` ;
  - le COPY du moteur : **forcé** (le défaut du moteur aujourd'hui) ou
    **journalisé** (`force_checkpoint_on_copy=false`, le défaut qu'on
    s'apprête à basculer).
- **En mode blobs**, rag3weaver repose lui-même le COPY forcé à chaque paquet
  (`code_sync.rs`, `commencer`). La colonne « journalisé » en blobs mesure
  donc la bascule telle qu'elle sera : moteur journalisé, et blobs forcés par
  le produit.
- **Conditions** : dépôt entier (7 031 fichiers), base neuve sur disque à
  chaque passe, une seule tenue `poste mesure`, de 15 h 4x à 16 h 57.
- **Calme** : il est atteint avant chacune des 12 passes (pression d'E/S
  avg10 sous 2 %, attente de 0 à 26 s). Aucune passe n'est écartée.

## Les passes

| Tour | Mode | COPY | Durée | Pic | Plus gros journal | Replis | 1re chose cherchable | Attente / pression |
|---|---|---|---|---|---|---|---|---|
| 1 | fichiers | forcé | 82 s | 9,2 Go | 0 Mo | 0 | 13 s | 0 s / 1,7 % |
| 1 | fichiers | journalisé | 81 s | 9,1 Go | 165 Mo | 0 | 12 s | 0 s / 0,3 % |
| 1 | blobs | forcé | 89 s | 14,0 Go | 0 Mo | 0 | 11 s | 0 s / 0,9 % |
| 1 | blobs | journalisé | 87 s | 14,0 Go | 0 Mo | 0 | 10 s | 18 s / 1,9 % |
| 2 | fichiers | forcé | 80 s | 9,0 Go | 0 Mo | 0 | 13 s | 10 s / 1,8 % |
| 2 | fichiers | journalisé | 79 s | 9,3 Go | 165 Mo | 0 | 11 s | 0 s / 0,7 % |
| 2 | blobs | forcé | 89 s | 13,7 Go | 0 Mo | 0 | 10 s | 0 s / 0,3 % |
| 2 | blobs | journalisé | 84 s | 13,9 Go | 0 Mo | 0 | 10 s | 26 s / 1,8 % |
| 3 | fichiers | forcé | 74 s | 9,2 Go | 0 Mo | 0 | 10 s | 10 s / 1,8 % |
| 3 | fichiers | journalisé | 86 s | 9,4 Go | 165 Mo | 0 | 12 s | 0 s / 0,3 % |
| 3 | blobs | forcé | 92 s | 14,0 Go | 0 Mo | 0 | 10 s | 0 s / 0,7 % |
| 3 | blobs | journalisé | 89 s | 13,9 Go | 0 Mo | 0 | 11 s | 12 s / 1,7 % |

Toutes les passes sont vertes. « Replis » est la valeur de
`copy_journal_fallbacks`. Sorties : `~/.cache/rag3weaver-build/mesure-sab-*.out`
et `serie-sab.out`.

## Ce que ça dit

| Mode | COPY forcé (médiane) | COPY journalisé (médiane) | Écart |
|---|---|---|---|
| fichiers | 82, 80, 74 → **80 s** | 81, 79, 86 → **81 s** | **+1 %**, dans le critère |
| blobs | 89, 89, 92 → **89 s** | 87, 84, 89 → **87 s** | **−2 %** : les blobs restent à leur temps |

- **Fichiers** : le COPY journalisé tient à quelques pour cent près. Ce matin,
  sur la lib qui fuyait, il coûtait +5 % ; c'est maintenant +1 %, dans le
  bruit des trois tours (74 à 86 s). Le journal pèse 165 Mo au plus, comme
  avant ; aucun repli.
- **Blobs** : le COPY forcé que pose rag3weaver fait son travail. Le journal
  reste vide (0 Mo, contre 461 Mo sans ce forçage le 5 octobre), et le temps
  est celui du COPY forcé.
- **Mémoire** : 9,0–9,4 Go en fichiers, 13,7–14,0 Go en blobs, quel que soit
  le COPY.
- **Ce que `fb98852e1` a fait gagner au défaut d'aujourd'hui** (COPY forcé,
  contre la référence du 5 octobre, moteur `eb2d78e46`, même forme) :
  - temps : rien de mesurable (fichiers 80 s contre 78 s ; blobs 89 s contre
    78–89 s) ;
  - mémoire : environ −0,5 Go en blobs (13,7–14,0 Go contre 14,2–14,7 Go),
    rien en fichiers.
- **La première chose cherchable** arrive entre 10 et 13 s.

**Le critère du basculement est rempli** : en fichiers, le COPY journalisé est
à +1 % ; en blobs, le forçage de rag3weaver les ramène à leur temps.
