# Des mains locales pour le gros modèle : DeepSeek en ligne, un petit modèle à luciepc qui édite et cherche

Ouvert le 27 novembre 2026. Lucie : passer les démos sur DeepSeek quand les crédits Gemini s'épuiseront (sans
investissement, ce sont les seules conditions réelles d'un produit au début) ; un petit modèle qui fait les vraies
actions (« édite la fonction… », « cherche-moi… »), lancé par l'agent au-dessus, comme ses mains ; un Qwen de 27B ou un
Qwen Coder, servi par son second PC quand elle ne s'en sert pas ; plusieurs sessions à la fois sans VRAM en plus, avec
un ordonnancement.

## 1. DeepSeek déclaré

- Une entrée dans `models.llm` (fournisseur compatible OpenAI, déjà pris en charge) et son prix dans `prices.json`,
  **lu sur la page officielle le jour de la pose** (les tarifs relevés le 27 novembre varient selon les sources, avec
  des heures pleines et creuses ; le prix d'une entrée servie par le cache est le plus important pour nous).
- La clé d'API est saisie par Lucie elle-même.
- Réserve écrite dans la déclaration : les données partent chez DeepSeek ; jamais pour du code client.

## 2. La main d'édition

- Le gros modèle donne une **intention structurée** : `{action: "edit", target: "<chemin>::<symbole>", instruction}` ;
  il ne lit plus le fichier entier et n'écrit plus le code (les jetons de sortie sont les plus chers).
- La main reçoit **le seul morceau visé**, tiré de l'index ligne pour ligne, la consigne et les signatures voisines ;
  elle rend le morceau réécrit.
- L'hôte l'applique, réindexe, lance la compilation et les tests du fichier ; le gros modèle reçoit le diff, court, et
  le résultat.
- **Après deux échecs** (ne compile pas, test rouge), la main rend la tâche au gros modèle avec l'erreur.
- En DAG : un nœud-graphe « Main » (lire le morceau → `LlmNode` du petit modèle → appliquer → vérifier → route réussi |
  réessayer | rendre), déclaré comme gabarit, modifiable comme les autres.
- La main de recherche est l'éclaireur de l'index (`2026-11-24-l-eclaireur-de-l-index.md`).

## 3. Le service de luciepc

- luciepc a deux Radeon AI PRO R9700 de 32 Gio (relevé du 27 novembre) ; card2 porte les services, card0 les écrans. Un
  27B en Q4_K_M (~16-17 Gio) tient sur card2 avec les services d'embarquement.
- **Un seul chargement des poids pour plusieurs sessions** : `llama-server --parallel N` (emplacements partagés,
  regroupement des requêtes au vol) ; une session ne coûte que son contexte. Les mains demandent peu de contexte :
  4 à 8 emplacements de 16 000 jetons.
- File d'attente de llama-server quand les emplacements sont pris ; sauvegarde et restauration du contexte d'une
  session au repos sur disque.
- Candidats à départager sur le banc, au taux de réussite des éditions : un Qwen dense de 27B et Qwen3-Coder 30B-A3B.
  Licences à lire à la version exacte (règle : Apache, MIT, BSD).

## 4. L'ordonnancement côté hôte

- **Priorité** : la session interactive de Lucie avant les quêtes de nuit.
- **Délai maximal d'attente** d'une main ; au-delà, bascule sur le modèle en ligne déclaré (DeepSeek ou Gemini), dit
  dans le journal du tour.

## 5. La disponibilité de luciepc

- Le service se déclare disponible ou non ; l'hôte n'y envoie les mains que s'il répond, sinon au modèle en ligne :
  aucune session ne reste bloquée parce que le PC est pris.
- D'abord à la main (« luciepc libre », « luciepc à moi ») ou par plage horaire ; plus tard, arrêt de lui-même quand la
  carte est demandée par autre chose (le verrou `poste gpu` sait qui la tient).

## Les témoins

Par un vrai hôte, rouge d'abord, avec un faux modèle pour la main : une intention d'édition sur un symbole indexé
produit un diff appliqué et réindexé ; deux échecs rendent la main au gros modèle avec l'erreur ; un service de mains
absent fait basculer sur le modèle en ligne, dit dans le journal ; deux sessions simultanées sont servies par le même
service sans second chargement.
