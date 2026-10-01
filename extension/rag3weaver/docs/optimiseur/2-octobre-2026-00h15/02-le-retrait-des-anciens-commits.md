# Les forks réécrits, et ce qui reste visible

**2 octobre 2026, session « optimiseur ».** Suite du [01](01-les-sept-pr-amont.md) :
les commits de nos forks portaient une adresse qui n'avait rien à faire dans
un dépôt public. Lucie a demandé la correction ; elle est faite, à une limite
près, qui est sa décision.

## Ce qui est fait

| fork | branche | commits recréés | diff ancien / nouveau | autre branche |
|---|---|---|---|---|
| `L-Defraiteur/burn` | `rag3weaver/pre.3`, sommet `21674205` | 3 | vide | `rag3weaver/pre.2` supprimée |
| `L-Defraiteur/cubecl` | `rag3weaver/pre.3`, sommet `bdf6b77a` | 1 | vide | `rag3weaver/pre.2` supprimée |
| `L-Defraiteur/cubek` | `rag3weaver/pre.3`, sommet `7ba8affd` | 2 | vide | `rag3weaver/pre.2` supprimée |

Les commits ont été recréés à l'identité personnelle de Lucie, posée en local
dans chaque clone (la configuration globale du poste n'a pas été touchée) :
mêmes diffs, mêmes messages, mêmes dates d'auteur. Poussés par ssh sous le
compte `L-Defraiteur`, avec `--force-with-lease=<branche>:<ancien sommet>`.
Les `pre.2` n'étaient épinglées dans aucun `Cargo.toml` ni `Cargo.lock`
d'aucune branche de rag3db.

L'épinglage de master suit (`04b5052ec`) : les 51 entrées `[patch.crates-io]`
de `extension/rag3weaver/Cargo.toml` et le `Cargo.lock`, par remplacement des
trois hashes et rien d'autre ; `cargo metadata --locked` prouve le lock.

## Ce qui reste visible

Les neuf anciens commits ne sont plus sur aucune branche ni aucun tag, mais
GitHub les sert encore **par leur hash** (vérifié par l'API le 2 octobre), et
pas seulement par l'URL des forks : aussi par celle des dépôts d'origine,
parce qu'un fork partage le magasin d'objets de son réseau. Le journal
d'activité des forks, relu, confirme qu'il n'y a que ces neuf-là.

Deux voies, et une seule est sûre :

- **demander la purge au support GitHub** : c'est la seule qui garantisse la
  disparition. La demande est rédigée, avec les neuf hashes complets, dans
  `.vault/demande-support-github-forks.md` (hors de git, exprès : ces hashes
  sont les poignées de ce qu'on veut voir disparaître). Lucie l'envoie
  elle-même, depuis le compte `L-Defraiteur` ;
- **supprimer et recréer les trois forks** : ne garantit rien, pour la raison
  ci-dessus (les objets vivent dans le réseau du dépôt d'origine).

Rien n'a été envoyé au support.

## Ce que ça change pour qui construit

Un commit de rag3db antérieur à `04b5052ec` épingle les anciennes révisions :
il continue de se construire sur un poste qui les a dans `~/.cargo/git`, et
tant que GitHub les sert ; plus après la purge. Les autres branches de rag3db
portent encore l'ancien épinglage : à remonter par leurs propriétaires
(même remplacement de trois hashes dans `Cargo.toml` et `Cargo.lock`, ou un
rebase sur master).

Les docs datés et trois messages de commit citent les anciens hashes en
abrégé : ils restent tels quels, comme références mortes.
