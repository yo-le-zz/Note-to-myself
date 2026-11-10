#!/usr/bin/env bash
# Compile Note to Myself et génère les paquets .deb et .rpm dans ./dist
#
# Usage :  ./build.sh            -> binaire + .deb + .rpm
#          ./build.sh deb        -> binaire + .deb seulement
#          ./build.sh rpm        -> binaire + .rpm seulement
#          ./build.sh bin        -> binaire seulement
#          ./build.sh arch       -> paquet Arch Linux (.pkg.tar.zst), nécessite makepkg et git
#          ./build.sh --test ... -> lance aussi les tests avant de compiler
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")"

RUN_TESTS=0
TARGETS=()
for arg in "$@"; do
  case "$arg" in
    --test) RUN_TESTS=1 ;;
    deb|rpm|arch|bin|all) TARGETS+=("$arg") ;;
    -h|--help) sed -n '2,10p' "$0"; exit 0 ;;
    *) echo "Argument inconnu : $arg (voir ./build.sh --help)" >&2; exit 1 ;;
  esac
done
[ ${#TARGETS[@]} -eq 0 ] && TARGETS=(all)
want() { for t in "${TARGETS[@]}"; do [ "$t" = all ] || [ "$t" = "$1" ] && return 0; done; return 1; }

command -v cargo >/dev/null || { echo "cargo est introuvable : installe Rust via https://rustup.rs" >&2; exit 1; }

NAME=note-to-myself
VERSION=$(cargo pkgid | sed 's/.*[#@]//; s/.*://')
echo "==> $NAME $VERSION"

if [ "$RUN_TESTS" = 1 ]; then
  echo "==> Tests"
  cargo test --locked
fi

echo "==> Compilation release"
cargo build --release --locked

rm -rf dist && mkdir -p dist
cp "target/release/$NAME" "dist/$NAME-$VERSION-linux-$(uname -m)"

ensure_tool() { # $1 = sous-commande cargo, $2 = crate
  if ! cargo "$1" --help >/dev/null 2>&1; then
    echo "==> Installation de $2 (une seule fois)"
    cargo install --locked "$2"
  fi
}

if want deb; then
  ensure_tool deb cargo-deb
  echo "==> Paquet .deb"
  cargo deb --no-build --no-strip -o "dist/${NAME}_${VERSION}_amd64.deb" >/dev/null
fi

if want rpm; then
  ensure_tool generate-rpm cargo-generate-rpm
  echo "==> Paquet .rpm"
  cargo generate-rpm -o "dist/${NAME}-${VERSION}-1.x86_64.rpm" >/dev/null
fi

if [ "${TARGETS[*]}" = "arch" ] || { want arch && [ "${TARGETS[*]}" != "all" ]; }; then
  command -v makepkg >/dev/null || { echo "makepkg est introuvable : ce paquet se construit sur Arch Linux" >&2; exit 1; }
  echo "==> Paquet Arch Linux"
  build_dir=$(mktemp -d)
  git archive --prefix="$NAME-$VERSION/" -o "$build_dir/$NAME-$VERSION.tar.gz" HEAD
  cp packaging/arch/PKGBUILD "$build_dir/"
  (cd "$build_dir" && makepkg --noconfirm --nocheck >/dev/null)
  cp "$build_dir"/*.pkg.tar.zst dist/
  rm -rf "$build_dir"
fi

(cd dist && sha256sum * > SHA256SUMS)
echo
echo "Terminé. Fichiers dans ./dist :"
ls -lh dist
echo
echo "Installer :  sudo apt install ./dist/${NAME}_${VERSION}_amd64.deb      (Debian/Ubuntu/Mint)"
echo "             sudo dnf install ./dist/${NAME}-${VERSION}-1.x86_64.rpm   (Fedora/openSUSE: zypper install)"
