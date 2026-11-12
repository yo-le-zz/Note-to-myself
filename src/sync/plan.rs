//! Décision de synchronisation, sans réseau ni base de données : on compare l'état local, l'état du pair
//! et le dernier état commun (`base`), comme le fait git pour une fusion à trois voies.

use crate::settings::SyncMode;
use std::collections::{BTreeSet, HashMap};

/// Résumé d'une note pour la comparaison.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Entry {
    pub uid: String,
    pub hash: String,
    pub updated_at: String,
    pub deleted: bool,
}

pub struct PlanInput<'a> {
    pub local: &'a [Entry],
    pub remote: &'a [Entry],
    /// Dernier état commun avec ce pair : uid → empreinte.
    pub base: &'a HashMap<String, String>,
    pub mode: SyncMode,
    pub local_is_primary: bool,
    pub remote_is_primary: bool,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Plan {
    /// Notes à envoyer au pair.
    pub push: Vec<String>,
    /// Notes à récupérer du pair.
    pub pull: Vec<String>,
    /// Notes à supprimer définitivement ici (mode miroir).
    pub drop_local: Vec<String>,
    /// Notes à supprimer définitivement chez le pair (mode push).
    pub drop_remote: Vec<String>,
    /// Notes modifiées des deux côtés, tranchées automatiquement.
    pub conflicts: Vec<String>,
    /// Nouvel état commun après exécution du plan.
    pub new_base: HashMap<String, String>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Side {
    Local,
    Remote,
}

/// `a` est-il strictement plus récent que `b` ? Les dates sont comparées comme des instants (RFC 3339 avec
/// leur fuseau) : comparer les textes donnerait un mauvais résultat entre deux PC de fuseaux différents.
/// Si une date est illisible, on retombe sur la comparaison des textes.
fn is_newer(a: &str, b: &str) -> bool {
    use chrono::DateTime;
    match (
        DateTime::parse_from_rfc3339(a),
        DateTime::parse_from_rfc3339(b),
    ) {
        (Ok(x), Ok(y)) => x > y,
        _ => a > b,
    }
}

/// Qui gagne un conflit : le PC principal, sinon la modification la plus récente
/// (à égalité, le pair, pour que le résultat soit stable).
fn winner(local: &Entry, remote: &Entry, input: &PlanInput) -> Side {
    match (input.local_is_primary, input.remote_is_primary) {
        (true, false) => return Side::Local,
        (false, true) => return Side::Remote,
        _ => {}
    }
    if is_newer(&local.updated_at, &remote.updated_at) {
        Side::Local
    } else {
        Side::Remote
    }
}

pub fn plan(input: &PlanInput) -> Plan {
    let local: HashMap<&str, &Entry> = input.local.iter().map(|e| (e.uid.as_str(), e)).collect();
    let remote: HashMap<&str, &Entry> = input.remote.iter().map(|e| (e.uid.as_str(), e)).collect();
    let uids: BTreeSet<&str> = local.keys().chain(remote.keys()).copied().collect();
    let mut plan = Plan::default();

    for uid in uids {
        let (l, r) = (local.get(uid).copied(), remote.get(uid).copied());
        let owned = uid.to_string();
        match input.mode {
            SyncMode::Mirror => match (l, r) {
                (_, Some(r)) => {
                    if l.map(|l| l.hash != r.hash).unwrap_or(true) {
                        plan.pull.push(owned.clone());
                    }
                    plan.new_base.insert(owned, r.hash.clone());
                }
                (Some(_), None) => plan.drop_local.push(owned),
                (None, None) => {}
            },
            SyncMode::Push => match (l, r) {
                (Some(l), _) => {
                    if r.map(|r| r.hash != l.hash).unwrap_or(true) {
                        plan.push.push(owned.clone());
                    }
                    plan.new_base.insert(owned, l.hash.clone());
                }
                (None, Some(_)) => plan.drop_remote.push(owned),
                (None, None) => {}
            },
            SyncMode::Union => match (l, r) {
                // Rien n'est modifié ni supprimé : on ne copie que les notes vivantes manquantes.
                (Some(l), None) if !l.deleted => {
                    plan.push.push(owned.clone());
                    plan.new_base.insert(owned, l.hash.clone());
                }
                (None, Some(r)) if !r.deleted => {
                    plan.pull.push(owned.clone());
                    plan.new_base.insert(owned, r.hash.clone());
                }
                (Some(l), Some(r)) if l.hash == r.hash => {
                    plan.new_base.insert(owned, l.hash.clone());
                }
                _ => {}
            },
            SyncMode::Newest => match (l, r) {
                (Some(l), Some(r)) if l.hash == r.hash => {
                    plan.new_base.insert(owned, l.hash.clone());
                }
                (Some(l), Some(r)) => {
                    if is_newer(&l.updated_at, &r.updated_at) {
                        plan.push.push(owned.clone());
                        plan.new_base.insert(owned, l.hash.clone());
                    } else {
                        plan.pull.push(owned.clone());
                        plan.new_base.insert(owned, r.hash.clone());
                    }
                }
                (Some(l), None) => {
                    plan.push.push(owned.clone());
                    plan.new_base.insert(owned, l.hash.clone());
                }
                (None, Some(r)) => {
                    plan.pull.push(owned.clone());
                    plan.new_base.insert(owned, r.hash.clone());
                }
                (None, None) => {}
            },
            SyncMode::Smart => match (l, r) {
                (Some(l), Some(r)) if l.hash == r.hash => {
                    plan.new_base.insert(owned, l.hash.clone());
                }
                (Some(l), Some(r)) => {
                    let base = input.base.get(uid);
                    let local_changed = base != Some(&l.hash);
                    let remote_changed = base != Some(&r.hash);
                    let side = match (local_changed, remote_changed) {
                        (true, false) => Side::Local,
                        (false, true) => Side::Remote,
                        _ => {
                            plan.conflicts.push(owned.clone());
                            winner(l, r, input)
                        }
                    };
                    match side {
                        Side::Local => {
                            plan.push.push(owned.clone());
                            plan.new_base.insert(owned, l.hash.clone());
                        }
                        Side::Remote => {
                            plan.pull.push(owned.clone());
                            plan.new_base.insert(owned, r.hash.clone());
                        }
                    }
                }
                // Présente d'un seul côté : ajout (ou note purgée chez l'autre, qu'on rétablit).
                (Some(l), None) => {
                    plan.push.push(owned.clone());
                    plan.new_base.insert(owned, l.hash.clone());
                }
                (None, Some(r)) => {
                    plan.pull.push(owned.clone());
                    plan.new_base.insert(owned, r.hash.clone());
                }
                (None, None) => {}
            },
        }
    }
    plan
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(uid: &str, hash: &str, updated: &str, deleted: bool) -> Entry {
        Entry {
            uid: uid.into(),
            hash: hash.into(),
            updated_at: updated.into(),
            deleted,
        }
    }

    fn live(uid: &str, hash: &str, updated: &str) -> Entry {
        entry(uid, hash, updated, false)
    }

    fn base(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs
            .iter()
            .map(|(a, b)| (a.to_string(), b.to_string()))
            .collect()
    }

    fn run(
        mode: SyncMode,
        local: Vec<Entry>,
        remote: Vec<Entry>,
        base: HashMap<String, String>,
        local_primary: bool,
        remote_primary: bool,
    ) -> Plan {
        plan(&PlanInput {
            local: &local,
            remote: &remote,
            base: &base,
            mode,
            local_is_primary: local_primary,
            remote_is_primary: remote_primary,
        })
    }

    #[test]
    fn identical_states_do_nothing() {
        let p = run(
            SyncMode::Smart,
            vec![live("a", "h", "1")],
            vec![live("a", "h", "1")],
            base(&[]),
            false,
            false,
        );
        assert!(p.push.is_empty() && p.pull.is_empty() && p.conflicts.is_empty());
        assert_eq!(p.new_base["a"], "h");
    }

    #[test]
    fn smart_propagates_adds_edits_and_deletes_both_ways() {
        let b = base(&[("edited", "h1"), ("deleted", "h2"), ("same", "h3")]);
        let local = vec![
            live("edited", "h1b", "2"), // modifiée ici
            live("deleted", "h2", "1"), // supprimée chez le pair
            live("same", "h3", "1"),
            live("added-here", "n1", "2"),
        ];
        let remote = vec![
            live("edited", "h1", "1"),
            entry("deleted", "tomb", "2", true),
            live("same", "h3", "1"),
            live("added-there", "n2", "2"),
        ];
        let p = run(SyncMode::Smart, local, remote, b, false, false);
        let sorted = |mut v: Vec<String>| {
            v.sort();
            v
        };
        assert_eq!(sorted(p.push), vec!["added-here", "edited"]);
        assert_eq!(sorted(p.pull), vec!["added-there", "deleted"]);
        assert!(p.conflicts.is_empty());
        assert_eq!(p.new_base["deleted"], "tomb");
    }

    #[test]
    fn primary_wins_conflicts_whoever_is_newer() {
        let b = base(&[("n", "h0")]);
        let local = vec![live("n", "hl", "9")];
        let remote = vec![live("n", "hr", "1")];
        // Le pair est le principal : il gagne, même s'il est plus ancien.
        let p = run(
            SyncMode::Smart,
            local.clone(),
            remote.clone(),
            b.clone(),
            false,
            true,
        );
        assert_eq!(
            (p.pull.as_slice(), p.push.len(), p.conflicts.len()),
            (&["n".to_string()][..], 0, 1)
        );
        // Nous sommes le principal : nous gagnons.
        let p = run(
            SyncMode::Smart,
            local.clone(),
            remote.clone(),
            b.clone(),
            true,
            false,
        );
        assert_eq!(
            (p.push.as_slice(), p.pull.len()),
            (&["n".to_string()][..], 0)
        );
        // Aucun principal : la plus récente gagne.
        let p = run(SyncMode::Smart, local, remote, b, false, false);
        assert_eq!(p.push, vec!["n"]);
        assert_eq!(p.conflicts, vec!["n"]);
    }

    #[test]
    fn edit_versus_remote_delete_is_a_conflict_resolved_by_primary() {
        let b = base(&[("n", "h0")]);
        let local = vec![live("n", "edited", "5")];
        let remote = vec![entry("n", "tomb", "4", true)];
        let p = run(SyncMode::Smart, local, remote, b, false, true);
        assert_eq!(p.pull, vec!["n"]);
        assert_eq!(p.new_base["n"], "tomb");
    }

    #[test]
    fn without_a_base_both_sides_differing_counts_as_conflict() {
        let p = run(
            SyncMode::Smart,
            vec![live("n", "a", "1")],
            vec![live("n", "b", "2")],
            base(&[]),
            false,
            false,
        );
        assert_eq!(p.conflicts, vec!["n"]);
        assert_eq!(p.pull, vec!["n"]);
    }

    #[test]
    fn mirror_makes_local_an_exact_copy() {
        let local = vec![
            live("keep", "x", "1"),
            live("old", "y", "1"),
            live("local-only", "z", "1"),
        ];
        let remote = vec![
            live("keep", "x2", "2"),
            live("old", "y", "1"),
            live("new", "w", "2"),
        ];
        let p = run(SyncMode::Mirror, local, remote, base(&[]), false, true);
        let mut pull = p.pull.clone();
        pull.sort();
        assert_eq!(pull, vec!["keep", "new"]);
        assert_eq!(p.drop_local, vec!["local-only"]);
        assert!(p.push.is_empty());
    }

    #[test]
    fn push_makes_the_peer_an_exact_copy() {
        let local = vec![live("a", "1", "1"), live("b", "2", "1")];
        let remote = vec![live("a", "1", "1"), live("c", "3", "1")];
        let p = run(SyncMode::Push, local, remote, base(&[]), true, false);
        assert_eq!(p.push, vec!["b"]);
        assert_eq!(p.drop_remote, vec!["c"]);
        assert!(p.pull.is_empty());
    }

    #[test]
    fn union_only_adds_missing_live_notes() {
        let local = vec![
            live("a", "1", "1"),
            live("conflict", "l", "1"),
            entry("gone", "t", "2", true),
        ];
        let remote = vec![
            live("b", "2", "1"),
            live("conflict", "r", "1"),
            live("gone", "alive", "1"),
        ];
        let p = run(SyncMode::Union, local, remote, base(&[]), false, false);
        assert_eq!(p.push, vec!["a"]);
        assert_eq!(p.pull, vec!["b"]);
        // Ni les conflits ni les suppressions ne sont touchés.
        assert!(p.conflicts.is_empty() && p.drop_local.is_empty() && p.drop_remote.is_empty());
        assert!(!p.new_base.contains_key("conflict") && !p.new_base.contains_key("gone"));
    }

    #[test]
    fn union_never_spreads_tombstones() {
        let p = run(
            SyncMode::Union,
            vec![entry("x", "t", "1", true)],
            vec![],
            base(&[]),
            false,
            false,
        );
        assert!(p.push.is_empty());
    }

    #[test]
    fn newest_picks_the_latest_edit() {
        let p = run(
            SyncMode::Newest,
            vec![live("a", "l", "2"), live("b", "l", "1")],
            vec![live("a", "r", "1"), live("b", "r", "2")],
            base(&[]),
            true,
            false,
        );
        assert_eq!(p.push, vec!["a"]);
        assert_eq!(p.pull, vec!["b"]);
    }

    #[test]
    fn plan_is_idempotent_once_applied() {
        // Après exécution, rejouer le plan sur l'état résultant ne doit plus rien changer.
        let b = base(&[]);
        let p1 = run(
            SyncMode::Smart,
            vec![live("a", "1", "1")],
            vec![live("b", "2", "1")],
            b,
            false,
            false,
        );
        let merged = vec![live("a", "1", "1"), live("b", "2", "1")];
        let p2 = run(
            SyncMode::Smart,
            merged.clone(),
            merged,
            p1.new_base,
            false,
            false,
        );
        assert!(p2.push.is_empty() && p2.pull.is_empty() && p2.conflicts.is_empty());
    }

    #[test]
    fn timestamps_are_compared_as_instants_not_as_text() {
        // 10:00 à +02:00 = 08:00 UTC, donc plus ancien que 09:30 UTC, bien que le texte soit « plus grand ».
        assert!(!is_newer(
            "2026-01-01T10:00:00+02:00",
            "2026-01-01T09:30:00+00:00"
        ));
        assert!(is_newer(
            "2026-01-01T09:30:00+00:00",
            "2026-01-01T10:00:00+02:00"
        ));
        assert!(is_newer(
            "2026-01-01T10:00:00.500+00:00",
            "2026-01-01T10:00:00+00:00"
        ));
        assert!(!is_newer(
            "2026-01-01T10:00:00+00:00",
            "2026-01-01T10:00:00+00:00"
        ));
        // Dates illisibles : repli sur le texte, sans panique.
        assert!(is_newer("b", "a"));
        let p = run(
            SyncMode::Newest,
            vec![live("n", "l", "2026-01-01T10:00:00+02:00")],
            vec![live("n", "r", "2026-01-01T09:30:00+00:00")],
            base(&[]),
            false,
            false,
        );
        assert_eq!(p.pull, vec!["n"], "the remote edit is the most recent one");
    }
}
