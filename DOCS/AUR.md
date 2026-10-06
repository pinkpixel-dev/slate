# Publishing Slate on the AUR

This is the checklist for getting `slate-editor` onto the AUR and keeping it updated. I wrote it for a first publish, so it covers the one-time account setup too.

The package is called `slate-editor` because `slate` was already taken by a Qt pixel art editor. Both install `/usr/bin/slate`, so the PKGBUILD lists `slate`, `slate-git`, and `slate-bin` as conflicts.

The PKGBUILD in this repo lives at `packaging/aur/PKGBUILD`. The AUR has its own git repo per package, and only two files go there: `PKGBUILD` and `.SRCINFO`.

## One-time setup

1. Log in at [aur.archlinux.org](https://aur.archlinux.org).

2. Make an SSH key just for the AUR (or reuse one you already have):

   ```bash
   ssh-keygen -t ed25519 -f ~/.ssh/aur -C "aur"
   ```

3. Copy the contents of `~/.ssh/aur.pub` into **My Account → SSH Public Key** on the AUR site and save. You'll need your AUR password to confirm the change.

4. Tell SSH to use that key for the AUR. Add this to `~/.ssh/config`:

   ```
   Host aur.archlinux.org
     IdentityFile ~/.ssh/aur
     User aur
   ```

5. Install the helper tools if you don't have them. `updpkgsums` comes from `pacman-contrib`, and `namcap` checks packages for common mistakes:

   ```bash
   sudo pacman -S --needed base-devel pacman-contrib namcap
   ```

## Publishing a release

The PKGBUILD downloads the source tarball GitHub makes for a version tag, so the tag has to exist on GitHub (and the repo has to be public) before any of this works.

1. Merge the release to `main`, then tag it and push the tag:

   ```bash
   git tag v0.7.0
   git push origin v0.7.0
   ```

2. Clone the package's AUR repo somewhere outside the Slate repo. The first time, Git warns that you cloned an empty repository. That's normal, and it's how you create the package.

   ```bash
   git clone ssh://aur@aur.archlinux.org/slate-editor.git ~/aur/slate-editor
   cd ~/aur/slate-editor
   ```

3. Copy the PKGBUILD in and fill in the checksum. `updpkgsums` downloads the tarball and replaces `'SKIP'` with the real SHA-256:

   ```bash
   cp /path/to/slate/packaging/aur/PKGBUILD .
   updpkgsums
   ```

4. Build and install it locally to make sure it works. This builds from scratch, so it takes a while:

   ```bash
   makepkg -si
   ```

   Then check it with namcap. Warnings about dependencies already pulled in by other dependencies are usually fine:

   ```bash
   namcap PKGBUILD
   namcap slate-editor-*.pkg.tar.zst
   ```

5. Generate `.SRCINFO`. The AUR reads this file, not the PKGBUILD, so it has to be regenerated every time the PKGBUILD changes:

   ```bash
   makepkg --printsrcinfo > .SRCINFO
   ```

6. Commit and push. The AUR only accepts the `master` branch:

   ```bash
   git add PKGBUILD .SRCINFO
   git commit -m "slate-editor 0.7.0"
   git push origin master
   ```

The package page shows up at `https://aur.archlinux.org/packages/slate-editor` right away.

7. Copy the PKGBUILD with the real checksum back into `packaging/aur/PKGBUILD` in the Slate repo, so the two don't drift apart.

## Updating for a new version

1. Tag and push the new version on GitHub.
2. In `~/aur/slate-editor`, set `pkgver` to the new version and `pkgrel` back to `1`.
3. Run `updpkgsums`, then `makepkg -si` to test.
4. Run `makepkg --printsrcinfo > .SRCINFO`.
5. Commit both files and push.

If only the packaging changes (say, a new dependency) and Slate's version doesn't, bump `pkgrel` instead of `pkgver`.

## Things to know

- Don't commit build output to the AUR repo. A `.gitignore` there with `*` followed by `!PKGBUILD`, `!.SRCINFO`, and `!.gitignore` keeps it clean.
- Keep `options=('!lto')`. makepkg enables LTO by default, and that breaks linking the C code the tree-sitter grammars bring in. See `ERRORS.md`.
- `check()` runs Slate's test suite, which roughly doubles the build time. Anyone who wants to skip it can run `makepkg --nocheck`.
- The runtime dependencies come from what the binary links and loads: `libxcb`, `libxkbcommon`, `libxkbcommon-x11`, `wayland`, and `vulkan-icd-loader` for the window and GPU, `fontconfig` for the editor font picker, and `xdg-desktop-portal` for the Open and Save dialogs. If GPUI Kit starts needing something else, it goes in `depends` too.
