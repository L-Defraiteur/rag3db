# Cahier des charges — tout le fichier est couvert

**30 août 2026, 6 h.** Écrit d'avance, pour qu'une autre session puisse
l'implémenter sans avoir à redécouvrir le raisonnement.

## 1. Le défaut d'aujourd'hui

`codeparsers` extrait des scopes — fonctions, classes, méthodes — et **jette
silencieusement tout le reste**. Trois conséquences, dont aucune ne se voit :

- Un fichier dont l'arbre tree-sitter est à 90 % en erreur **compte comme un
  succès** avec zéro scope. `ProjectStats::successful_files` compte les
  fichiers qui n'ont pas levé d'exception, pas ceux qu'on a compris.
- `root_node().has_error()` **n'est jamais consulté**. `validate_node` et
  `extract_node_issues` existent dans la base et ne sont appelés nulle part.
  `ast_valid` n'est rempli que par deux parseurs sur sept, et personne ne le
  lit.
- « Zéro scope extrait » est **indiscernable** de « ce fichier n'a pas de
  fonctions » et de « on ne sait pas parser ce langage ».

Un moteur de recherche qui perd du texte sans le dire ment par omission — et
c'est la famille de défauts que ce dépôt passe ses journées à débusquer.

## 2. Ce qu'on veut, en une phrase

> **L'union des scopes d'un fichier couvre le fichier entier.** Toujours.
> Certains scopes sont du code compris ; les autres sont du texte qu'on n'a pas
> su rattacher, et ils le disent.

C'est l'idée de Lucie, et elle est meilleure qu'une métrique de couverture :
une métrique rend un **nombre** — « 41 % couvert » — et on ne sait toujours pas
*quoi* a été perdu. Un scope rend un **lieu**, qu'on peut lire, chercher et
citer.

### Ce que ça transforme

| | avant | après |
|---|---|---|
| couverture | une mesure a posteriori | un **invariant**, vérifiable par un test |
| texte non compris | perdu | indexé, cherchable, situé |
| « on parse mal ce langage » | une intuition | une donnée, par fichier |
| fichier non-code (Markdown, config) | hors du système | un scope de texte, comme le reste |

Le dernier point est celui que Lucie a vu venir : *« peut-être même ça pourrait
servir pour tous les fichiers textes »*. Un fichier dont on ne connaît pas le
langage n'est plus un cas particulier — c'est un fichier avec **un** scope.

## 3. La spécification

### 3.1 Deux genres de scope de plus

`ScopeInfoType` gagne deux variantes, et leur distinction compte :

| variante | ce que ça veut dire |
|---|---|
| `TexteNonRattache` | le parseur a compris le fichier, mais ce passage n'appartient à aucune construction extraite : commentaires de tête, `use`, code au niveau du fichier, lignes vides entre deux fonctions |
| `TexteNonCompris` | le parseur a échoué **ici** : l'arbre porte un `ERROR` couvrant ce passage |

Les confondre effacerait l'information qui compte. Le premier est **normal** —
tout fichier en a. Le second est un **aveu**, et c'est lui qu'on agrège pour
décider si un langage est mal servi.

### 3.2 L'invariant, et sa forme testable

Pour tout fichier analysé :

```
⋃ { [s.scope_start_byte, s.scope_end_byte) | s ∈ scopes de premier niveau }
    ==  [0, taille_du_fichier)
```

Sans trou et sans recouvrement. Les scopes imbriqués (une méthode dans une
classe) ne comptent pas : seuls les scopes de premier niveau pavent le fichier.

**C'est cette égalité qui doit être un test**, sur un corpus de fichiers réels
et sur des fichiers volontairement cassés.

### 3.3 Comment on découpe ce qui reste

Après extraction normale, on prend le complément des scopes de premier niveau
et on le découpe en **passages contigus maximaux**. Chaque passage devient un
scope, avec :

- `r#type` : `TexteNonCompris` s'il intersecte un nœud `ERROR`, sinon
  `TexteNonRattache` ;
- `name` : dérivé de la position — `«fichier».txt:120-158` — parce qu'un scope
  sans nom ne se cite pas ;
- `content` : le texte, tel quel ;
- `signature` : vide. Il n'y en a pas, et en inventer une serait mentir.

**Les passages blancs ne comptent pas.** Un passage qui ne contient que des
espaces et des sauts de ligne est rattaché au scope précédent plutôt que de
produire un scope vide — sinon un fichier normalement formaté produirait autant
de scopes de bruit que de fonctions.

### 3.4 Ce que le fichier déclare

`ScopeFileAnalysis` gagne une couverture **dérivée**, jamais saisie à la main :

```rust
pub struct Couverture {
    /// `root_node().has_error()` — ce que tree-sitter dit de lui-même.
    pub arbre_en_erreur: bool,
    /// Nombre de nœuds `ERROR` dans l'arbre.
    pub noeuds_erreur: usize,
    /// Octets du fichier.
    pub octets: usize,
    /// Octets dans un scope **de code** (ni `TexteNonRattache` ni
    /// `TexteNonCompris`).
    pub octets_code: usize,
    /// Octets dans un scope `TexteNonCompris`.
    pub octets_non_compris: usize,
}
```

`octets_code + octets_non_rattache + octets_non_compris == octets`, et c'est
la forme arithmétique du même invariant.

### 3.5 Le langage inconnu, et le fichier non-code

Un fichier dont l'extension n'a pas de parseur ne doit plus être **ignoré**. Il
produit une analyse avec **un** scope `TexteNonRattache` couvrant tout, et
`arbre_en_erreur: false` — on n'a pas échoué à le parser, on n'a pas essayé.
La différence est réelle et doit se voir.

C'est ce qui fait entrer Markdown, TOML, YAML, `.sh` et le reste dans le même
mécanisme, sans une seconde chaîne d'ingestion.

## 4. Ce que ça ne doit pas devenir

**Un index noyé de bruit.** Le risque principal, et il faut le nommer : un
fichier de 20 000 lignes qu'on parse mal produirait un scope de 20 000 lignes,
qui pèserait dans la recherche vectorielle comme un document entier et sortirait
devant les vraies fonctions.

Trois garde-fous, à décider mais à ne pas oublier :

1. **Un passage non compris se découpe** comme un document ordinaire — le
   `Chunker` existe et fait déjà ça pour les bases de connaissances.
2. **Le genre du scope doit être filtrable** à la recherche, et le rester dans
   le rendu : un résultat `TexteNonCompris` ne doit pas se présenter comme une
   fonction.
3. **La pondération** : un passage non compris ne devrait pas battre une
   fonction sur une requête qui nomme une fonction. À mesurer, pas à supposer.

## 5. Ce qui rendrait la chose fausse

À vérifier avant de conclure que ça marche :

- **Le double comptage.** Les scopes de premier niveau se recouvrent-ils déjà
  aujourd'hui ? Si oui, l'invariant est faux avant même qu'on ajoute quoi que
  ce soit, et c'est un défaut existant à corriger d'abord.
- **Les offsets sont à la granularité de la ligne** (`finalize` les dérive des
  lignes). Une construction qui commence au milieu d'une ligne casserait le
  pavage. À vérifier sur du code réel, pas sur des exemples.
- **L'encodage.** Les offsets sont en octets ; un fichier non-UTF-8 doit être
  refusé franchement plutôt que produire des tranches invalides.
- **Les fichiers binaires** qui traînent dans un dépôt : ils ne doivent pas
  devenir un scope de texte de 4 Mo.

## 6. Ce que ça permet ensuite

C'est la question de départ de Lucie : *« pouvoir à terme choisir les langages
qu'on expose vraiment »*.

Avec la couverture par fichier, la décision devient une lecture :

```
rust    89 fichiers   0 en erreur    97 % de code
cpp     31 fichiers   4 en erreur    62 % de code
go       7 fichiers   0 en erreur    91 % de code
tsx     12 fichiers  12 en erreur     8 % de code   ← à traiter comme du texte
```

Et « traiter comme du texte » n'est plus un travail : c'est déjà ce qui se
passe, on ne fait que cesser de prétendre le contraire.
