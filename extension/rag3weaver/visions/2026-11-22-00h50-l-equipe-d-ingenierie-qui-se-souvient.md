# L'équipe d'ingénierie qui se souvient

*Vision produit, 22 novembre 2026. Notée d'un échange de Lucie avec un autre assistant, relu et trié.*

## La phrase

« Votre équipe d'ingénierie d'agents, avec sa mémoire collective, ses outils, ses procédures et ses contrôles, qui
continue à progresser d'une session à l'autre. »

Ce n'est pas « un meilleur agent de code ». Ce qu'on vend, c'est de pouvoir confier un projet logiciel à une
organisation d'agents qui en garde l'histoire, comprend ses contraintes et travaille sous contrôle.

## Les phrases du produit (Lucie et c7, 22 nov. au soir)

La tension à dire : beaucoup d'agents qui travaillent seuls, partout, et pourtant on comprend tout.

> **Lâchez une équipe d'agents sur vos projets. Vous garderez le fil.**

- la plus directe : « Ils travaillent seuls. Vous savez toujours où ils en sont. »
- avec le mot de Lucie : « Une horde d'agents au travail, et vous comprenez tout. »
- pour la nuit de travail : « Vos agents travaillent toute la nuit. Le matin, vous savez tout. »
- pour la mémoire : « L'équipe d'agents qui se souvient. »

« Plusieurs machines, plusieurs équipes, plusieurs projets » va dans la ligne du dessous, comme preuve, pas dans la
promesse.

### Le leitmotiv : la ruche (Lucie, 22 nov., 22 h 50)

> **Become the hive mind. Control entire swarms of agents.**

« Des swarms d'agents, appelons ça swarm plutôt qu'équipe, c'est plus stylé, ou hive. » L'accent : on ne contrôle
pas une équipe d'agents, mais plusieurs. Le vocabulaire qui en sort :

- un **swarm** : un groupement d'agents sur un but, chacun avec son rôle, mené par un orchestrateur (ce que nous
  appelions « équipe ») ;
- la **hive** : tous les swarms de quelqu'un, sur toutes ses machines et tous ses projets ;
- le **hive mind** : la personne. Elle commande plusieurs swarms et comprend tout.

Les phrases françaises du dessus restent, comme lignes d'appui.

## Ce qui existe déjà et fait la différence

Chaque brique prise seule a des concurrents (Devin, Cursor, Factory…). La valeur, c'est l'intégration :

- **un savoir vivant** : graphe de code, recherche hybride, mémoire longue signée, un index par dépôt et des vues par
  worktree qui suivent les écritures ;
- **une organisation du travail** : plusieurs agents, des défauts tenus en tickets dans le dépôt, une file de lots
  qui n'atterrit que verte (le pousseur), des reprises d'une session à l'autre ;
- **une autonomie gouvernée** : bac, accords explicites, juges, carte humaine qui ne se cache jamais ;
- **une maîtrise des coûts** : registre des dépenses, plafond, prix datés ;
- **une indépendance** : nos propres moteurs (graphe, plein texte), le modèle au choix.

## Trois produits possibles

| Produit | Pour qui | Ce qu'il apporte |
|---|---|---|
| Mémoire et intelligence du code | équipes de développement, éditeurs d'IDE | comprendre durablement de grands dépôts |
| Orchestrateur d'agents | développeurs avancés, petites équipes | plusieurs agents sans conflits |
| Forge logicielle autonome | entreprises, R&D | déléguer des chantiers entiers, tracés, coûts tenus |

Les deux premiers peuvent se vendre seuls, avant le troisième.

## La vraie inconnue : la reproductibilité

Ça marche chez Lucie. Pour convaincre, il faut le montrer **sur des dépôts inconnus**, avec d'autres équipes et
d'autres modèles, en mesurant trois choses : **le coût total**, **les régressions**, **le nombre d'interventions
humaines**. Piste : un banc « dépôt étranger » (un dépôt public qu'aucune session n'a vu, un chantier donné, ces
trois chiffres relevés), rejoué à chaque version.

Lucie est sa propre première cliente, en usage intensif : chaque défaut de son travail réel nourrit le produit. C'est
un avantage, à condition de le mesurer.

## Le piège

Une architecture sophistiquée ne suffit pas. Un développeur extérieur doit pouvoir **installer, comprendre ce qui
se passe, et confier quelque chose d'important** sans devenir administrateur de la forge. D'où : une installation
en une commande, un accueil qui explique, des défauts lisibles, des procédures écrites pour un inconnu.

## Les connecteurs, après la stabilité

Dans l'ordre : git et les dépôts locaux, quelques fournisseurs de modèles, MCP, GitHub et GitLab. Ensuite seulement
Slack, Jira, Linear, Google Drive.

## Le chemin vers les premiers clients (Lucie, 22 nov.)

Rendre la forge installable par un inconnu demandera de l'investissement, ou bien **d'ouvrir d'abord une
infrastructure minimale** :

- **chez nous, l'inférence** : les hôtes qui servent les modèles (juges, embarquement, survivant) tournent sur nos
  machines, mutualisés ;
- **chez eux, le savoir** : le code, l'index, les mémoires et les accords restent sur leurs postes, sous leur
  contrôle.

Ce partage règle à la fois le coût d'entrée (pas de GPU à acheter pour essayer) et la confiance (leur code et leur
mémoire ne quittent pas leur machine). Il suppose des hôtes d'inférence joignables à distance et sûrs (auth, quota par
client, registre des dépenses par client), et que rien de leur savoir ne transite au-delà de ce qu'un appel de modèle
demande.

## Le modèle économique : une marge sur l'inférence (Lucie, 22 nov.)

L'inférence passe par nous, et nous prenons une marge sur chaque fournisseur, au choix du client :

- **Européen** pour ceux qui le veulent : Scaleway pour l'hébergement, Mistral pour les modèles ;
- **beaucoup d'autres fournisseurs** pour ceux à qui c'est égal (Gemini, Anthropic, OpenAI…) ;
- **Modal ou Hugging Face** seulement quand c'est indispensable : nos modèles ouverts qu'on ne trouve servis nulle
  part ailleurs (juges, embarquement).

Le registre des dépenses (prix datés, sources citées, plafonds) en est la base : il suffit d'y ajouter la marge et le
client. Le choix du fournisseur reste une ligne de config, comme aujourd'hui `models.llm`.

## Les offres : abonnement, marge et licence Community (Lucie, 22 nov.)

Deux revenus, plus une porte d'entrée gratuite :

- **un abonnement mensuel** pour profiter de nos systèmes : mémoire collective, orchestration, recherche, gouvernance
  des agents, coordination hébergée ;
- **la marge sur l'inférence** (section précédente), pour qui passe par notre facturation ;
- **une licence Community** : gratuite sous conditions, infrastructure et clés à la charge de l'utilisateur.

| Offre | Modèle |
|---|---|
| Community | licence gratuite sous conditions, sa propre infrastructure et ses propres clés |
| Pro | abonnement + consommation d'inférence |
| Team | abonnement par équipe + consommation ; mémoire partagée, gouvernance |
| Enterprise | contrat sur mesure, déploiement privé, contrôle des données, support |

Points à concevoir :

- **les clés du client (BYOK)** : abonnement sans marge sur l'inférence. Le revenu ne dépend donc pas que de la
  revente de jetons ;
- **la Community comme canal d'acquisition** : ne pas enfermer trop tôt la mémoire et la recherche derrière le
  paiement, c'est elles qui font adopter. Le payant vend l'exploitation : synchronisation entre postes, coordination
  hébergée, équipes, garanties. « On distribue les outils de la forge, on facture son exploitation » ;
- **du registre à la facturation client** : isolation des comptes, attribution sûre de chaque appel, erreurs et
  remboursements, prix historiques, rapprochement avec les factures des fournisseurs ;
- **l'offre européenne** : distinguer l'hébergement européen, le traitement européen et le fournisseur européen dans
  ce qu'on garantit.
