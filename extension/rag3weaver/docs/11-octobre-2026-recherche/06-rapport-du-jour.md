# Session recherche — la journée du 11 octobre

*Pour Lucie et pour l'orchestration. Tout ce qui suit est sur master, sauf ce
qui est dit en vol à la fin.*

## Le lot 0 des agents qui se parlent : l'exécuteur de flux

Les ports portent des flux (décision de Lucie). La page de conception est la
05 de ce dossier ; la 04 tient les pas 1 et 2.

| quoi | commit | témoins |
|---|---|---|
| Le saut d'une branche non prise, dit au rapport | `522fa6780` | rouge d'abord, en direct et avec points de reprise |
| Les déclarations (`PortKind`, `Posture`) et les sorties requises respectées | `feeb1932b` | une sortie requise non posée est une faute nommée |
| La frontière d'un sous-graphe garde le genre et la posture | `fd10b8c27` | `Value` au lieu d'`Open`, rouge d'abord |
| Les genres résolus (`PortDef::follows`, `dataflow::kinds`) et `next` / `emit` sous les niveaux | `9521ab63c` | le témoin 12 côté niveaux |
| L'exécuteur concurrent (canaux bornés maison, interblocage nommé, annulation, `cancel_handle`, `token_sink`) | `439ca3c9c` | 10 témoins et 12 du canal ; le premier rouge sous les niveaux |
| Les points de reprise d'un graphe à flux (un flux en ports numérotés) | `3f13b1657` | le témoin 9, rouge sous les niveaux |
| Un sous-graphe relaie les flux (3a) | `eb49aba8d` | le témoin 10, rouge avec l'ancien `GraphNode` |

La batterie complète du pas 3 tourne sur `eb49aba8d`, une fois, hors carte
locale. Reste le **pont vivant (3b)** : un sous-graphe qui relaie ses flux en
direct, ce qui lève le refus d'un `Open` sur son entrée. La page passera
avant le code.

La session du proto a relu chaque pas. Elle porte le côté nœud : `Route` et
`Decide` en flux, et bientôt `LlmNode` qui émet ses jetons.

## Master rouge sous un jeu de features, et le contrôle qui en est né

- `7eb3af52e` : `e2e_mesure_joignabilite` employait `common::burn` sans
  demander `burn-embedder`. Dans le binaire unique, tout le binaire cassait.
  Ticket de G ouvert et fermé dans le même commit.
- `scripts/check_features.sh` vérifie `cargo check --tests` sous chaque jeu
  employé (cinq depuis l'ajout de `rag3db-native` seul par l'arbre principal).
  L'orchestration en a fait le contrôle avant tout hash.

## La garde du target

Cargo juge frais l'artefact d'un paquet local sur des chemins relatifs et sur
les dates. Un target partagé entre deux arbres reprend donc les artefacts de
l'autre (constat de codeparsers). `scripts/garde_target.sh` (`2306d30f9`, puis
`d432b56d3` après le constat de G) note l'arbre dans le target et refuse un
autre arbre. La reprise efface les empreintes des cinq paquets locaux, dans
chaque profil **et chaque triplet** (`cargo clean -p` laissait le triplet),
puis le vérifie. Témoin : `scripts/test_garde_target.sh`.

## Les relectures

Pour la session du proto : les lots 2 (`Decide`, `Route`, `schema_for`) et 3
(`send`, l'annuaire), le gabarit de `Decide`, la montre de fichiers (W1, W2 :
c'est le disque qui décide, pas l'événement), le retard dit (W3 : un `done`
qui n'efface pas un changement plus récent, ni un fichier mis en reprise),
puis `Route` et `Decide` en flux. Chaque fois, les défauts trouvés ont été
corrigés avec leur témoin avant le push.

## Les leçons

- **Un push pendant un rebâti de la lib** : j'ai arrêté le mien quand
  l'arbre principal a demandé la lib ; il avait déjà rebasé sans pousser.
  Je l'ai relancé après l'annonce des md5.
- **Un témoin sensible à l'ordre des fils** (le 9) : il a été vu rouge par
  le rejeu sous le verrou de push, pas par moi. Il attend désormais
  l'annulation, et il a passé cinq fois de suite.
- **Le sous-module d'un worktree rebasé** reste au vieux pointeur :
  `git submodule update` avant le check (mémoire écrite).

## En vol

- La batterie complète du pas 3 sur `eb49aba8d`.
- La correction de `Route` et `Decide` en flux chez le proto : un lecteur
  qui lâche (`NoReader`) ne doit pas faire échouer le nœud.
