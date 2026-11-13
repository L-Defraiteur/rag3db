# Reprendre le week-end — point d’entrée

Passation demandée par Lucie le 13 novembre 2026 parce que la limite d’usage approche.
**Les quatorze sessions ont un rapport autonome. Les branches WIP ont été poussées,
même inachevées. Aucun nouveau calcul n’est autorisé par cette passation.**

Commencer par la [synthèse d’orchestration](99-passation-week-end.md), puis le rapport
du chantier repris. Les copies ci-dessous viennent de commits identifiés, pas d’une
fusion des branches entières. Les travaux encore en cours restent sur leurs branches.

## Priorités de reprise

1. Couches 2b-A : application `d9aa4be86c6f21665cf85c22e47134c87ab6aab8`,
   dernier pilote `4fca0dd38378d5bb31269dab76713d6bc0060ebc`. Quatre avis statiques
   favorables et simulations hermétiques ; **aucune exécution réelle de ce pilote**.
   Anciennes qualifications rouges conservées. Reprendre après inspection et un
   nouveau budget explicite ; aucune échéance ancienne ne se réactive.
2. Lucivy : correctif fonctionnel B qualifié et mesuré, mais coût trop élevé dans
   sept cas sur huit. Alternative atomique C `fa1d0273c468b997163f3cf9d5f56bae052f522a`
   poussée et relue statiquement ; **NON JOUÉE**. Qualifier C avant toute mesure A/C.
   Le rouge CI statique et son détenteur historique restent non attribués ; pas de release.
3. Régularisations documentaires prêtes : clôture du reliquat Recherche `a6c23af93fa60cf064f1491ac90babb45e1055c3`
   et clôture du ticket minimum tty (lot complet `e593c67a2`, pas son premier commit
   partiel `0882f0a86`). Ces clôtures sont poussées sur leurs branches, pas fusionnées
   par la seule collecte des rapports.

## Rapports et sauvegardes

| Session | Rapport | Branche distante du rapport | Commit source |
|---|---|---|---|
| Orchestration | [Synthèse et règles](99-passation-week-end.md) | `cx-orchestration-passation-weekend-r1` | commit de cet index |
| arbre-principal | [Passation](../../arbre-principal/13-11-2026/99-passation-week-end.md) | `cx-arbre-passation-week-end-2026-11-13` | `495d11907ea0` |
| lucivy | [Passation](../../lucivy/13-11-2026/99-passation-week-end.md) | `cx-lucivy-passation-weekend` | `942fc34f8b1f` |
| recherche | [Passation](../../recherche/13-11-2026/99-passation-week-end.md) | `cx-recherche-passation-weekend-13nov-r1` | `9cb6de90ada1` |
| everything-declarative | [Passation](../../everything-declarative/13-11-2026/99-passation-week-end.md) | `cx-ed-ci-writer-diagnostic-r1` | `73977a2bc451` |
| memoire | [Passation](../../memoire/13-11-2026/99-passation-week-end.md) | `cx-memoire-passation-weekend-r1` | `8ff6bba85927` |
| rag3weaver-archi | [Passation](../../rag3weaver-archi/13-11-2026/99-passation-week-end.md) | `cx-archi-passation-week-end-20261113` | `9d6ead2bc0a4` |
| coeur-cpp | [Passation](../../coeur-cpp/13-11-2026/99-passation-week-end.md) | `cx-coeur-passation-weekend` | `04b92a1e464f` |
| optimiseur | [Passation](../../optimiseur/13-11-2026/99-passation-week-end.md) | `cx-optimiseur-passation-weekend` | `13e69050a465` |
| embarquement | [Passation](../../embarquement/13-11-2026/99-passation-week-end.md) | `cx-embarquement-passation-week-end-20261113` | `f4d791a0956b` |
| banc | [Passation](../../banc/13-11-2026/99-passation-week-end.md) | `cx-banc-residence-appels` | `6df3ff2bcb87` |
| coeur-cpp-hotfixs-tickets | [Passation](../../coeur-cpp-hotfixs-tickets/13-11-2026/99-passation-week-end.md) | `cx-hotfixs-passation-week-end-20261113` | `0eb6ded42eac` |
| tty | [Passation](../../tty/13-11-2026/99-passation-week-end.md) | `cx-docs-tty-features` | `a20ad187ceef` |
| codeparsers | [Passation](../../codeparsers/13-11-2026/99-passation-week-end.md) | `cx-codeparsers-passation-weekend` | `2753df2ceaed` |

Les SHA complets et chemins sont dans [le manifeste des sources](passations-sources.json).
Les treize têtes de branches ont été vérifiées sur origin sans écart :
[contrôle des références distantes](passations-remotes.json). Les rapports recensent
aussi les branches du produit ; la branche du rapport peut contenir du runtime
historique qui ne doit pas être fusionné avec le document.

## Retrouver un travail sans toucher à l’arbre de Lucie

```bash
git fetch origin
git show <SHA-du-rapport>:docs/<session>/13-11-2026/99-passation-week-end.md
git worktree add -b cx-reprise-<lot> <nouveau-chemin-prive> <SHA-source-exact>
```

C7 désigne le rôle de coordination, pas une session qu’il faudrait retrouver.
La session qui reprend l’orchestration peut l’assumer dans le périmètre déjà
autorisé par Lucie. Un nouveau GO signifie une recette/source/budget cohérents,
pas une permission humaine à demander pour chaque action ordinaire déjà autorisée.
Ne pas recréer toutes les anciennes sessions pour obtenir des avis rituels.

Lire `docs/README.md`, les tickets canoniques et `docs/procedures/push-d-un-lot.md`
avant toute exécution. Le portable est `/home/lucied/git_workspaces/rag3db` ;
les calculs se font à `luciepc` par SSH, dans des worktrees et targets privés.
Ne pas remplacer les SHA par des têtes flottantes pour attribuer des preuves.
Les journaux et ELF massifs ne sont pas envoyés dans Git : chemins, manifestes
et copies sur les deux postes sont explicités dans les rapports.

L’arbre de Lucie garde ses deux modifications préexistantes : sous-module
`extension/rag3weaver/codeparsers` et dossier non suivi
`extension/rag3weaver/docs/lucie-tmp/`. Aucune sauvegarde de cette passation ne
prétend les avoir intégrés ou supprimés.
