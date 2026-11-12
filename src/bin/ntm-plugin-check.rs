//! Vérifie un ou plusieurs dossiers de plugin (utilisé par la CI des pull requests).
//!
//! Usage : `ntm-plugin-check plugins/mon-plugin [autres dossiers…]`
//! Code de sortie : 0 si tout est valide, 1 sinon.

use ntm_core::plugin::{Hook, Plugin};
use std::path::Path;
use std::process::ExitCode;

fn main() -> ExitCode {
    let folders: Vec<String> = std::env::args().skip(1).collect();
    if folders.is_empty() {
        eprintln!("usage: ntm-plugin-check <plugin-folder>...");
        return ExitCode::from(2);
    }
    let mut failed = false;
    let mut seen_ids = std::collections::BTreeSet::new();
    for folder in &folders {
        match Plugin::load_dir(Path::new(folder)) {
            Ok(plugin) => {
                let id = plugin.manifest.id.clone();
                if !seen_ids.insert(id.clone()) {
                    eprintln!("FAIL {folder}: duplicate plugin id `{id}`");
                    failed = true;
                    continue;
                }
                if Path::new(folder).file_name().and_then(|n| n.to_str()) != Some(id.as_str()) {
                    eprintln!("FAIL {folder}: the folder name must equal the plugin id `{id}`");
                    failed = true;
                    continue;
                }
                println!(
                    "OK   {folder}: {} v{} — risk {:?}",
                    plugin.manifest.name,
                    plugin.manifest.version,
                    plugin.analysis.risk()
                );
                for permission in &plugin.analysis.permissions {
                    println!(
                        "       permission {} ({})",
                        permission.id(),
                        permission.describe()
                    );
                }
                for hook in &plugin.analysis.hooks {
                    let name = match hook {
                        Hook::Start => "on_start",
                        Hook::NoteSave => "on_note_save",
                    };
                    println!("       hook {name}");
                }
                for warning in &plugin.analysis.warnings {
                    println!("       warning: {warning}");
                }
            }
            Err(errors) => {
                for error in errors {
                    eprintln!("FAIL {folder}: {error}");
                }
                failed = true;
            }
        }
    }
    if failed {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}
