<p align="center">
  <img src="packaging/icons/note-to-myself-256.png" alt="Note to Myself" width="128">
</p>

<h1 align="center">Note to Myself</h1>

<p align="center">Une pensée, rien que pour toi.<br>Une application de notes minimaliste pour Linux, écrite en Rust.</p>

## Fonctionnalités

- Écrire, modifier et supprimer des notes
- Liste épurée : numéro de la note à gauche, titre, et un petit bouton pour dérouler le texte
- Menu par note : Modifier, Copier le texte, Supprimer (avec confirmation, désactivable)
- Recherche dans les titres et le contenu
- Interface traduite en 20 langues, changement immédiat
- Thème sombre ou clair, taille du texte réglable, affichage compact
- Raccourci Ctrl+S pour enregistrer
- 100 % local : base SQLite, aucun compte, aucune connexion réseau
- Migration automatique et sans perte des anciennes bases

## Langues

English, Français, Español, Deutsch, Italiano, Português, Nederlands, Polski, Русский, Українська, Türkçe, العربية, 日本語, 한국어, 中文, हिन्दी, Svenska, Ελληνικά, Română, Čeština.

Les traductions sont dans `src/i18n.rs` : pour corriger un texte ou ajouter une langue, il suffit d'y ajouter un tableau.

## Installation

Télécharge le paquet correspondant à ta distribution depuis les releases, ou génère-le toi-même (voir plus bas).

```sh
# Debian, Ubuntu, Linux Mint
sudo apt install ./note-to-myself_0.1.0_amd64.deb

# Fedora
sudo dnf install ./note-to-myself-0.1.0-1.x86_64.rpm

# openSUSE
sudo zypper install ./note-to-myself-0.1.0-1.x86_64.rpm
```

## Compiler

Il faut [Rust](https://rustup.rs) (édition 2021 ou plus récente).

```sh
cargo run --release
```

### Générer les paquets .deb et .rpm

```sh
./build.sh          # binaire + .deb + .rpm dans ./dist
./build.sh deb      # binaire + .deb
./build.sh rpm      # binaire + .rpm
./build.sh bin      # binaire seulement
./build.sh --test   # lance aussi les tests avant de compiler
```

Au premier lancement, le script installe `cargo-deb` et `cargo-generate-rpm` (connexion internet nécessaire).

## Données

Les notes et les réglages restent sur ta machine :

- notes : `~/.local/share/notetomyself/notes.sqlite3`
- réglages : `~/.config/notetomyself/settings.json`

Pour tout sauvegarder, copie ces deux fichiers.

## Structure du projet

```
src/main.rs      point d'entrée, gestion des messages
src/app.rs       état de l'application, base de données, migration
src/i18n.rs      traductions
src/ui/          écrans (accueil, écriture, notes, réglages)
packaging/       icônes et fichier .desktop
build.sh         compilation et génération des paquets
```

Construit avec [iced](https://iced.rs), [rusqlite](https://github.com/rusqlite/rusqlite) et [Lucide](https://lucide.dev) pour les icônes.

## Licence

MIT, voir le fichier [LICENSE](LICENSE). Créé par yolezz.
