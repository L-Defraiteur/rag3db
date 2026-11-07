//! **Le terminal de l'agent de code** (extension/rag3weaver/docs/07-novembre-2026-tty).
//!
//! Lot tty-1 : le client du flux, sans interface. L'adresse et le jeton de
//! l'hôte ([`endpoint`]), les appels JSON ([`client`]), le flux SSE de
//! `/ide/flux` avec sa reprise ([`sse`], [`flux`]), et le rendu en texte brut
//! d'un événement ([`plain`]). Un crate à part : rien ici ne dépend du moteur : le binaire tourne
//! là où est l'hôte, ou sur une autre machine, à côté de lui par un tunnel
//! (Windows, macOS, Linux).

pub mod client;
pub mod endpoint;
pub mod flux;
pub mod plain;
pub mod sse;
