//! **Le pont navigateur** (15 novembre 2026) : l'hôte, en Rust, de l'extension Chrome d'un fork de
//! jackwener/browsermcp (révision 5 de son protocole). Chrome lance l'hôte par Native Messaging ;
//! l'extension tient le débogueur, les onglets et la page ; l'hôte lui envoie des commandes typées.
//!
//! - [`cadrage`] : une trame = une longueur sur 4 octets (petit-boutiste) puis du JSON UTF-8.
//! - [`protocole`] : les enveloppes et leurs types.
//! - [`pont`] : la corrélation par `id`, `hello` puis `ready`, le délai, la fermeture.
//! - [`baux`] : les onglets de chaque session, et la garde du passage de main.
//!
//! Deux gardes de l'hôte s'appliquent avant tout envoi : un onglet rendu à l'humain ne se reprend
//! que par un nouveau `claim`, et les passe-partout (`cdp`, `chrome-call`…) sont fermés tant
//! qu'une [`pont::Politique`] ne les ouvre pas. Conception :
//! `docs/rag3weaver-archi/15-11-2026/04-conception-coeur-pont-navigateur.md`.

pub mod baux;
pub mod cadrage;
pub mod ecoute;
pub mod hote_natif;
pub mod pont;
pub mod protocole;
pub mod relai;

#[cfg(test)]
mod temoins;
#[cfg(test)]
mod temoins_relai;
