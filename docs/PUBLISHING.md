# Publishing

How releases reach users. Everything is driven by pushing a `v*` tag:

```bash
git tag v0.1.0
git push origin v0.1.0
```

`.github/workflows/release.yml` then builds the Windows installer and the `.deb`,
publishes them as a GitHub release together with `SHA256SUMS`, updates the Scoop
manifest and opens a winget pull request. `.github/workflows/apt-repo.yml`
rebuilds the APT repository whenever a release is published.

The one-time setup for each channel is below.

## 1. Prerequisites

| Setting | Where | Value |
| --- | --- | --- |
| Pages source | Settings → Pages → Source | GitHub Actions |
| `PUBLISH_APT` | Settings → Variables → Actions | `true` |
| `PUBLISH_WINGET` | Settings → Variables → Actions | `true` |
| `WINGET_TOKEN` | Settings → Secrets → Actions | GitHub PAT, see below |
| `APT_GPG_PRIVATE_KEY` | Settings → Secrets → Actions | armoured private key, see below |
| `APT_GPG_PASSPHRASE` | Settings → Secrets → Actions | passphrase, or omit if the key has none |

The `PUBLISH_*` variables act as kill switches: the winget and APT jobs are
skipped entirely until you set them to `true`.

## 2. winget

Installs with `winget install ArveLomsland.SyncthingStatus`.

**The first version must be submitted by hand.** Automation can only update a
package that already exists in `microsoft/winget-pkgs`.

1. Publish release `v0.1.0` and copy the installer hash from `SHA256SUMS`.
2. Fill in `InstallerSha256` in
   `packaging/winget/ArveLomsland.SyncthingStatus.installer.yaml`.
3. Validate and submit:

   ```powershell
   winget install wingetcreate
   wingetcreate submit --token <PAT> packaging\winget
   ```

   Alternatively, fork `microsoft/winget-pkgs` and copy the three files to
   `manifests/a/ArveLomsland/SyncthingStatus/0.1.0/`, then open a pull request.
4. A bot validates the manifest and installs the package in a sandbox. Review by
   a maintainer usually takes a few days.

Afterwards, set `PUBLISH_WINGET=true` and every tag opens the update PR
automatically.

`WINGET_TOKEN` must be a **classic** personal access token with the `public_repo`
scope, created on the account that owns the `winget-pkgs` fork.

> The installer is not code signed, so Windows SmartScreen may warn on first run.
> winget does not require signing, but signing removes the warning.

## 3. APT repository

Installs with `apt install syncthing-status` after a one-time source setup.

Generate a signing key (no passphrase keeps CI simplest):

```bash
gpg --batch --quick-gen-key "SyncthingStatus Repository <arve.lomsland@proplan.no>" \
    default default never
gpg --list-secret-keys --keyid-format=long
gpg --armor --export-secret-keys <KEYID>    # -> secret APT_GPG_PRIVATE_KEY
```

Paste the whole block, including the `-----BEGIN/END-----` lines, into the
secret. Set `PUBLISH_APT=true` and set Pages to build from GitHub Actions.

The workflow collects the `.deb` from **every** release, so old versions stay
installable, and it republishes the whole index on each run. Users install with
the commands shown in the README and on the published Pages site.

> Back up the private key. If it is lost, every user has to re-add a new key
> manually before `apt update` works again.

## 4. Scoop

Installs with:

```powershell
scoop bucket add syncthingstatus https://github.com/ArveLomsland/SyncthingStatus
scoop install syncthing-status
```

No review process. `bucket/syncthing-status.json` starts at version `0.0.0` with
a placeholder hash and is rewritten by the `scoop` job on the first release, so
do not advertise the bucket before then.

## 5. Not set up

| Channel | Why not |
| --- | --- |
| Official Debian/Ubuntu | Needs a sponsoring Debian maintainer; months to years |
| Launchpad PPA | Source-only builds require `debian/rules` plus `cargo vendor` |
| Flathub | Needs an AppStream metainfo file and offline Cargo sources |
| Snap | Reasonable next step once the Linux build is actually tested |
| Homebrew | macOS has no event loop implemented yet |
