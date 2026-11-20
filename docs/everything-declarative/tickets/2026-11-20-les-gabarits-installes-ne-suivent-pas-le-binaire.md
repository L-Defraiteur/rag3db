# Les gabarits installés ne suivent pas le binaire

- **État** : ouvert (20 novembre 2026)
- **Famille** : produit (les gabarits, l'hôte)
- **Gravité** : grave. Les hôtes de Lucie servent la fusion d'il y a cinq semaines, quel que soit le binaire : rien de
  ce qui a atterri dans `templates/` depuis le 11 octobre n'atteint le service. C'est pourquoi le juge du banc sur la
  copie repassée de G est rouge (5/10, 43 %) : `FuseResultsNode` y est servi sans `dominant`, sans `exact_name` et sans
  `exact_name_leaf`.
- **Atteignable en service** : oui, sur toutes les bases installées avant le 8 novembre 2026.
- **Touche rag3weaver** : oui (`src/builtin_templates.rs`, l'ouverture d'un backend).
- **Pour** : everything declarative
- **Signalé par** : le banc (20 novembre 2026, soir), relayé par c7.

## Ce qu'on voit

Les manifestes de Lucie (par exemple `~/.config/rag3weaver/code-rag3db/templates/backends/code/rag3db.json`) pointent
vers des **copies locales** des gabarits (`templates/tools/*.mmd`), datées du 11 octobre. Rien ne les rafraîchit.

## Ce qui existe déjà, et pourquoi ça ne suffit pas

Le 19 octobre, `src/builtin_templates.rs` a apporté :
- les références `builtin:<chemin>` d'un manifeste, qui suivent chaque bâti ;
- `drift` : une copie locale qui diffère du livré se **dit** à l'ouverture ;
- `templates link` : une commande qui remplace par `builtin:` les copies identiques au livré **courant**.

Le 8 novembre, `record` et `refresh` ont suivi : un manifeste **posé d'après un gabarit livré** est noté, et remis au
gabarit du binaire s'il n'a pas été modifié.

Mais :
- **un manifeste posé avant le 8 novembre n'a pas de note** : `refresh` ne fait rien (« pas posé d'après un gabarit
  livré ») ;
- **une copie identique à un gabarit livré d'une version antérieure** n'est pas reconnue comme telle : elle diffère du
  livré courant, donc `link` la garde, et `drift` ne fait que la dire ;
- **rien ne tourne seul** : `link` est une commande à lancer.

## Ce qu'il faut pour le fermer

À l'ouverture d'une base :
- une copie installée **identique à un gabarit livré d'une version antérieure** du binaire est remplacée par le gabarit
  du binaire, et le journal le dit ;
- une copie **modifiée par l'utilisateur** est gardée, et le journal le dit ;
- `/sante` et l'état donnent la version des gabarits servis.

Pour reconnaître une version antérieure, le binaire doit connaître les empreintes de tous les gabarits livrés avant
lui. Une table d'empreintes tirée de l'historique de git, commitée et vérifiée par un témoin qui exige que l'empreinte
du gabarit courant y soit, le permet sans dépendre de git au moment de l'ouverture.

Rouge d'abord : un gabarit installé d'une vieille version est servi tel quel à l'ouverture.
