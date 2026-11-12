//! Cœur de Note to Myself : chiffrement, stockage, thèmes, plugins et synchronisation.
//! Cette bibliothèque ne dépend pas de l'interface graphique, ce qui permet de la tester seule.

pub mod crypto;
pub mod i18n;
pub mod notes;
pub mod plugin;
pub mod settings;
pub mod store;
pub mod sync;
pub mod theme;
pub mod vault;
