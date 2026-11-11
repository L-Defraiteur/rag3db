# Donner des commandes à un agent, sans lui donner la machine

> **Copie de conception du 30 août 2026**, remontée ici parce que c'est une
> pièce de la feuille de route et pas seulement un choix technique : c'est ce
> qui décide si l'agent de code peut fermer sa boucle. L'original reste dans
> `docs/30-aout-2026-04h00/03-…` avec la session qui l'a produit.
>
> **Depuis :** `auto` devient le mode de premier rang, et non `standard` — voir
> [doc 09 §2](09-trois-roles-et-une-seule-main.md). Les modes ne concernent que
> le rôle `code`. Et `codeparsers::shell` réduit désormais une ligne en argv,
> ce que le §1 tenait pour hors de portée.

**30 août 2026.** Le modèle, à qui on demandait son avis, l'a mis en première
place et deux fois : *« je code à l'aveugle »*, *« un agent qui ne peut pas
tester ses modifications est un agent qui produit du code cassé »*. Lucie :
*« faut trouver maintenant comment lui donner accès aux commandes système,
peut-être avec une allowlist de trucs qui n'ont pas besoin d'humain »*.

Ce document conçoit le mécanisme. Il ne le décide pas seul : les valeurs par
défaut sont proposées, pas imposées.

## 1. La décision qui protège vraiment : **pas de shell**

Avant toute liste, avant tout classifieur.

Une commande s'exécute par son **argv**, jamais par un interpréteur.
`["cargo", "test", "--lib"]`, pas `"cargo test --lib"`. Sans cette règle, toute
liste blanche est décorative : `cargo test; rm -rf ~` commence par `cargo`, et
un préfixe autorisé laisse passer ce qui le suit.

Conséquences assumées :

- Pas de `|`, `>`, `&&`, `$(…)`, `~`, `*`. Ce sont des services du shell, pas
  du programme appelé.
- Qui veut vraiment un shell demande `sh -c` **explicitement**, et c'est une
  commande comme une autre — qui ne sera jamais dans une liste blanche, parce
  que sa famille ne dit rien de ce qu'elle fait.

Le prix : un agent qui veut rediriger une sortie devra demander le fichier au
lieu du chevron. C'est un prix, et il est petit devant l'alternative.

## 2. Les trois modes

Ils suivent ceux d'un assistant de code, que Lucie connaît :

| mode | la liste blanche | le reste |
|---|---|---|
| `standard` | **lecture seule uniquement**, sans rien demander | refusé, sans demander |
| `approbation` | tourne librement | **demande à l'humain** |
| `auto` | tourne librement | une **sentinelle** tranche |

`standard` est le défaut. Un agent qui ne fait que lire ne peut rien casser, et
n'interrompt personne — c'est le régime dans lequel on veut le laisser tourner
sans surveillance.

**`refuse` n'est pas `demande`.** En `standard`, une commande hors liste est
refusée *et le dit*, avec la liste — la même règle que `RootPolicy` : « qui dit
non avec la liste ». Un refus muet enverrait l'agent réessayer autrement.

## 3. La sentinelle, et sa sortie

Une **sentinelle** juge une commande. Elle est enfichable : la première est un
jeu de règles sans modèle, la seconde interrogera un petit modèle, et rien
n'empêche d'en écrire une troisième.

```rust
pub trait Sentinelle: Send + Sync {
    fn juger(&self, commande: &Commande, contexte: &Contexte) -> Verdict;
}
```

Ce qu'un verdict doit porter tient en quatre morceaux, et **chacun répond à une
question qu'on se posera plus tard** :

```rust
pub struct Verdict {
    pub decision: Decision,   // qu'est-ce qu'on fait maintenant ?
    pub portee: Portee,       // ça vaut pour quoi d'autre ?
    pub fondement: Fondement, // sur quoi ça repose ?
    pub faits: Faits,         // qu'a-t-on observé ?
    pub motif: String,        // à dire à l'humain
}
```

### `decision` — maintenant

`Autorise` · `Demande` · `Refuse`. Trois, pas deux : « demande » n'est pas un
refus poli, c'est un état où quelqu'un doit trancher.

### `portee` — pour quoi d'autre

**C'est la pièce qui empêche de redemander cinquante fois.** Lucie : *« ne pas
redemander 50 fois à l'user si ok pour lire la bdd si donné une fois »*.

| portée | ce que ça couvre |
|---|---|
| `CetteFois` | cet appel, et lui seul |
| `CetteCommande` | le même argv exactement, pour la session |
| `CetteFamille` | la même famille (`cargo test`), pour la session |
| `Toujours` | la famille, écrite dans la configuration |

Une portée large ne s'invente pas : elle se **dérive du fondement**. Un « oui »
d'humain sur `cargo test --lib a` peut valoir `CetteFamille` ; un jugement
d'innocuité ne vaut jamais plus que `CetteCommande`, parce qu'il n'a pas
d'autorité — il a un avis.

### `fondement` — sur quoi ça repose

| fondement | d'où ça vient |
|---|---|
| `Configuration` | l'opérateur l'a écrit avant la session |
| `UtilisateurExplicite` | quelqu'un a dit oui, à cette chose, dans cette session |
| `DejaAccorde` | une portée acquise plus tôt couvre ce cas |
| `JugeeInoffensive` | personne n'a rien dit ; la sentinelle estime |

**Séparer les deux derniers est le cœur du dispositif.** Une commande peut
tourner parce qu'on l'a permise, ou parce qu'elle *semble* anodine — et ce ne
sont pas les mêmes risques. Les confondre, c'est perdre la trace du moment où
un humain s'est engagé.

### `faits` — ce qu'on a observé

```rust
pub struct Faits {
    pub ecrit: bool,          // modifie des fichiers
    pub reseau: bool,         // sort de la machine
    pub hors_domaine: bool,   // touche hors du domaine de travail
    pub irreversible: bool,   // suppression, force, réécriture d'historique
    pub eleve: bool,          // sudo, doas, root
    pub shell: bool,          // sh -c, bash -c : le contenu échappe à l'analyse
}
```

**Pourquoi stocker les faits et pas seulement la décision.** Une décision
enregistrée sans ses raisons ne se rejoue pas : le jour où la politique change,
on ne peut ni re-trancher ni auditer. Avec les faits, une trace ancienne
répond encore à « pourquoi a-t-on laissé passer ça ». C'est la même raison qui
fait garder `meta.warnings` plutôt qu'un booléen « ça s'est bien passé ».

## 4. La liste blanche que la session se construit

Lucie : *« peut-être en fait il crée sa propre allowlist, le classifieur,
durant une session »*.

Une `Session` d'autorisations garde les portées acquises. Chaque appel la
consulte **avant** la sentinelle : si une portée couvre le cas, le verdict est
`Autorise` avec `fondement: DejaAccorde`, sans rien demander à personne et sans
appeler de modèle.

Trois propriétés qu'on veut :

1. **Elle ne survit pas à la session.** Une permission accordée pendant un
   travail ne doit pas s'appliquer au suivant, sauf à être écrite dans la
   configuration — ce qui est un geste, pas un effet de bord.
2. **Elle ne s'élargit jamais toute seule.** `CetteFamille` acquis sur
   `cargo test` ne donne rien sur `cargo publish` : la famille, c'est le
   programme **et** le premier sous-verbe, pas le programme seul. `git` n'est
   pas une famille ; `git status` en est une.
3. **Ce qui est irréversible n'est jamais allowlistable.** `rm`, `git push
   --force`, `DROP` : même après un « oui », la portée reste `CetteFois`. On
   ne demande pas cinquante fois pour lire ; on demande à chaque fois pour
   détruire.

## 5. Ce que la sentinelle de base sait faire, sans modèle

Une liste de familles en lecture seule (`ls`, `cat`, `git status`, `git diff`,
`git log`, `cargo check`, `cargo test`, `rg`, `find`…), une liste de faits
détectés par motif (`sudo`, `--force`, `rm`, `curl`, `>`), et la règle des
portées ci-dessus. Pas de modèle, pas de latence, pas de dépendance.

**C'est elle le défaut**, et c'est délibéré : un mécanisme de sûreté qui a
besoin d'un modèle distant pour dire non a un mode de panne de trop.

La sentinelle à modèle vient au-dessus, pour le mode `auto`, et son verdict est
**contraint par la même structure** — un modèle ne peut pas inventer une
décision hors des trois, ni une portée plus large que ce que son fondement
autorise. Elle propose ; la structure borne.

## 6. Ce qui n'est pas décidé

- **Où vit la configuration** des familles `Toujours`. Un fichier de projet
  serait cohérent avec `.rag3weaver/`, mais une permission d'exécution dans un
  dépôt partagé se propage à qui le clone. À trancher.
- **Le format d'une demande à l'humain** quand il n'y a pas de terminal — le
  mode `approbation` suppose quelqu'un pour répondre.
- **Le délai d'expiration** d'une portée de session : aucune pour l'instant.
