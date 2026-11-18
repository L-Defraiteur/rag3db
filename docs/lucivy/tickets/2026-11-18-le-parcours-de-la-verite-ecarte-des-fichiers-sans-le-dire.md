# Le parcours de la vérité écarte des fichiers sans le dire

- **État** : ouvert (18 novembre 2026). Il remplace le ticket de Codex
  `2026-11-16-l-admission-du-corpus-fs-ne-garantit-pas-la-vue-consommee.md`, resté sur sa branche
  `cx-lucivy-reprise-16nov`. Son harnais de 1 274 lignes (branches lucivy `cx-lucivy-fs-*`) n'est pas repris
  (`docs/lucivy/18-11-2026/01-evaluation-du-travail-de-codex.md`).
- **Gravité** : faible. La comparaison reste juste, puisque l'index et la vérité sautent les mêmes fichiers. Mais un
  vert ne dit pas sur combien de fichiers il porte, ni qu'il en reste.
- **Atteignable en service** : non (tests seulement).
- **Touche rag3weaver** : non.
- **Famille** : produit
- **Pour** : lucivy

## Ce qui se passe

`lucivy_core/tests/test_sfx_v3_ground_truth.rs`, la fonction `walk` (lucivy main `a1c0dfd`) :
- l. 111 : un dossier illisible est sauté (`Err(_) => return`) ;
- l. 125 : un fichier non UTF-8 est sauté (`Err(_) => continue`) ;
- l'ordre est celui de `read_dir`, non trié, et le plafond `V3_MAX_DOCS` (5 000) coupe ce parcours non trié. Le
  sous-ensemble retenu peut donc changer d'un système de fichiers à l'autre.

`test_derived_in_ram.rs` (l. 24 et 35) fait de même.

## Ce qu'il faut faire

- Trier les entrées de chaque dossier avant de les parcourir. La coupe à `V3_MAX_DOCS` devient alors stable.
- Compter les dossiers et les fichiers écartés, par raison, et l'imprimer avec le nombre de fichiers retenus.
- Refuser une vue vide, par la garde existante `common::usable`.

Ne pas construire d'admission à reçus, de registres ni de SHA : c'est ce que la pile de Codex a fait, et elle n'est
pas reprise.

## Comment le prouver

- Un témoin rouge sur un dossier temporaire : un fichier non UTF-8 et un sous-dossier illisible doivent apparaître
  dans le compte des écartés, et deux parcours du même dossier doivent rendre la même liste dans le même ordre.
- Puis la suite, sur le corpus `linux-7.2`, avec la ligne du compte relevée.
