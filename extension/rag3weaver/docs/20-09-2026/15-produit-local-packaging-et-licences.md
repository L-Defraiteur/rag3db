# Produit local Magic : distribution et première vérification des licences

Date : 20-09-2026. Étude préliminaire technique et documentaire ; aucune autorisation juridique obtenue, aucun avis juridique définitif. Aucun changement du collecteur ou publication npm effectué.

## Architecture demandée

L'utilisateur installe lui-même MTGA Tool. Notre produit importe ses exports sans distribuer ce logiciel ni reprendre son code. Le moteur, la recherche et l'assistance à la construction de decks s'exécutent localement. Un paquet npm peut fournir un SDK et lancer le moteur natif compilé ; une interface de bureau peut utiliser le même SDK.

Cette architecture reste à réaliser. Actuellement `experiments/mtga/package.json` dépend de `mtga-reader` et `scripts/collect.cjs` le charge avec `require('mtga-reader')` pour lire la mémoire du processus MTGA. Le simple déplacement de son installation chez l'utilisateur ne transforme pas cette intégration en import de fichiers indépendant.

La disponibilité d'un export MTGA Tool complet, stable et couvrant quantités, decks, textes et capacités reste à vérifier. Les données actuellement utilisées proviennent de plusieurs sources locales : lecteur mémoire, journal et bases du client Arena. Ne pas promettre qu'un seul export existant remplace déjà ce circuit.

## GPL : séparation réelle, pas seulement installation séparée

Les projets [MTGA Tool](https://github.com/mtgatool/mtgatool-desktop) et [mtga-reader](https://github.com/mtgatool/mtga-reader) indiquent GPL-3.0. La GPLv3, section 2, n'étend pas automatiquement sa licence à toutes les sorties d'un logiciel : leur contenu doit lui-même constituer une œuvre couverte. Cela soutient l'architecture d'un importeur indépendant de données, sans y inclure du code GPL. [Texte GPLv3](https://www.gnu.org/licenses/gpl.en.html#section2).

La FSF distingue aussi programmes séparés et programme combiné selon le mécanisme et la nature des échanges. Un processus ou une API séparés ne garantissent pas à eux seuls l'indépendance si le couplage est intime. Importer des fichiers documentés, réutilisables avec plusieurs sources, constitue ici la séparation proposée ; ce n'est pas un avis définitif sur une future implémentation. [FAQ GNU](https://www.gnu.org/licenses/gpl-faq.en.html#MereAggregation).

Conséquence technique : le produit à distribuer ne devrait pas charger le module natif GPL actuel dans son processus si l'objectif est de s'appuyer sur cette séparation. Écrire notre propre adaptateur de données ne signifie pas recopier le parseur ou le collecteur GPL. Le paquet final et ses dépendances transitives devront correspondre à l'architecture retenue.

## Wizards : question distincte

Les conditions générales de Wizards, section 2.2, comportent des restrictions sur l'extraction automatisée non autorisée, l'ingénierie inverse et certains outils ou connexions tiers. Leur application exacte à un importeur local indépendant et aux méthodes de son fournisseur de données demande une analyse du montage réel et du droit applicable. Installer le fournisseur séparément n'accorde aucune permission supplémentaire. Aucune autorisation Wizards spécifique à notre produit ou à la collecte mémoire n'a été établie dans cette recherche. [Conditions Wizards](https://company.wizards.com/en/legal/terms).

La Fan Content Policy est une permission conditionnelle visant notamment du contenu gratuit ; elle ne constitue pas une autorisation générale de vendre du contenu Magic. Les textes et illustrations ne deviennent pas libres de droits parce qu'ils viennent de fichiers locaux. Un logiciel payant d'analyse mérite donc une qualification spécifique avant vente, plutôt que de s'appuyer uniquement sur cette politique. [Politique Wizards](https://company.wizards.com/en/legal/fancontentpolicy).

Conclusion provisoire : import local indépendant favorable à la séparation GPL ; droit d'exploitation commerciale et conditions d'extraction restant à clarifier. Pour une vente France/UE, faire examiner précisément ces deux points par un juriste logiciel/PI. La présence de trackers commerciaux n'établit pas les permissions dont ils disposent, ni celles de ce projet.

## Npm et protection contre la copie

Complément sur les produits existants : [Untapped.gg Premium](https://mtga.untapped.gg/premium) commercialise notamment l'assistance Draftsmith et des analyses statistiques. Sa page annonce aussi un export CSV de collection, à étudier comme connecteur indépendant. La mention de non-affiliation à Wizards ne permet pas de déduire l'existence ou l'absence d'un accord. Ce précédent commercial confirme un marché, sans établir notre propre autorisation.

[Overwolf indique travailler avec les éditeurs](https://support.overwolf.com/support/solutions/articles/9000182312-overwolf-won-t-get-you-banned) et impose une [validation des projets utilisant ses API](https://dev.overwolf.com/ow-native/guides/game-compliance/overview/). Cela illustre une voie d'intégration encadrée ; ce n'est pas une preuve d'un accord Wizards applicable à notre produit.

Le SDK JavaScript peut lancer un binaire Rust/C++ par plateforme, avec un protocole local versionné. Cela conserve l'API moteur et isole un crash natif. Les modèles peuvent se télécharger séparément et être mis en cache. Il reste à livrer toutes les bibliothèques/extensions nécessaires, retirer les chemins absolus du poste de développement et gérer le cycle de vie de la base.

Un paquet privé restreint son accès au registre ; il n'empêche pas un destinataire de copier ses fichiers. [Documentation npm](https://docs.npmjs.com/about-private-packages/). Une archive ASAR Electron est un format d'archive ; ses mécanismes d'intégrité détectent certaines altérations et ne constituent pas une garantie anti-crack. [Archives Electron](https://www.electronjs.org/docs/latest/tutorial/asar-archives), [intégrité ASAR](https://www.electronjs.org/docs/latest/tutorial/asar-integrity).

Recommandation d'architecture : binaire compilé, versions et mises à jour signées, licence signée vérifiable localement pour les composants propriétaires concernés, avec politique hors ligne explicite. Une clé privée de signature ne doit pas être embarquée. Le chiffrement du code exécuté chez l'utilisateur ne peut pas garantir son secret face au propriétaire de la machine. Garder une fonction côté serveur en protège mieux l'implémentation, mais change le coût, la confidentialité et la promesse locale du produit.

## Travail produit restant

| Priorité | Travail | État/justification |
|---|---|---|
| Bloquant | Intégrité persistante | L'incident des capacités imbriquées après réouverture, note 13, reste ouvert. Corriger, reconstruire puis auditer. |
| MVP | Import fiable | Sources détectées, formats versionnés, compte/snapshot explicites, différences et suppressions, reprise après échec et mises à jour Arena. |
| MVP | Installation autonome | Binaire prêt à lancer, dépendances natives, chemins portables, modèles, choix GPU, budgets RAM/disque et erreurs exploitables. Le plafond actuel du pool est 15 Gio, pas une mesure de consommation. |
| MVP | Agent de construction | Les décisions des decks expérimentaux ont été préparées avec l'assistant et des scripts spécifiques. Il faut une orchestration produit répétable, un fournisseur LLM configurable et des limites de coût/temps. |
| MVP | Vérifications des decks | Format/légalité à jour, possession et versions rééquilibrées, budget de jokers, mana conditionnel, contraintes utilisateur, explications sourcées. La similarité dense ne prouve pas un combo. |
| MVP | Atelier de decks | Brouillons persistants, versions retenues, comparaison, annulation, cartes verrouillées et export Arena. Les briques génériques ne constituent pas encore cet écran complet. |
| Bêta | Tests d'installation et endurance | Installation sur machine propre, fermeture brutale, réouverture, migrations, mises à jour et désinstallation. |
| Ensuite | Suivi des parties | Associer résultats et version exacte du deck, retours du joueur, propositions mesurables. Pas de garantie de taux de victoire. |

Appréciation, sans chiffrage ferme : encapsulation npm de difficulté modérée ; produit autonome de difficulté nettement supérieure, principalement à cause de la fiabilité, de la distribution native/GPU et de l'agent à formaliser. Commencer par un OS et un import de fichier explicite réduit le périmètre de la première bêta.

Références locales : [verbes moteur](14-verbes-de-recherche-dans-le-moteur.md), [audit des decks et incident de persistance](13-revision-des-13-decks-et-audit-des-donnees.md).
