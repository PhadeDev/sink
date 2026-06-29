# Arch packaging

`sink-bin` is the Arch package for Sink.
It repackages the official release `.deb` rather than building from source.

The `.deb` already lays files out under `/usr` and links against system PipeWire and webkit2gtk, so Arch users get a normally integrated install (launcher entry, icon, `pacman` uninstall) with no build toolchain and no bundled libraries.

## How the pipeline works

The `archpkg` job in `.github/workflows/release.yml` runs after every release:

1. Builds the package in an `archlinux:latest` container with `makepkg`.
2. Stamps the version and the real `sha256` of the released `.deb` into `PKGBUILD`.
3. Attaches the resulting `sink-bin-<version>-x86_64.pkg.tar.zst` to the GitHub release.

No AUR account is required: the prebuilt package is installed directly with `pacman -U`.

## Installing (for users)

Download `sink-bin-*-x86_64.pkg.tar.zst` from the release and install it:

```bash
sudo pacman -U ./sink-bin-*-x86_64.pkg.tar.zst
```

Uninstall with `sudo pacman -R sink-bin`.

## Building it yourself

On any Arch machine, with the `PKGBUILD` and a real release version:

```bash
sed -i 's/^pkgver=.*/pkgver=0.1.15/' PKGBUILD   # an existing release
updpkgsums
makepkg -si
```

## Publishing to the AUR later

The same `PKGBUILD` is AUR-ready.
AUR account registration was disabled when this was set up, so the package is shipped on the release instead.
When registration reopens, create the `sink-bin` AUR package and push this `PKGBUILD` (plus a generated `.SRCINFO`); the build job can then publish on each release too.
