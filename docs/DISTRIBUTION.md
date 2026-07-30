# Robrix Distribution

Robrix releases use native installers rather than treating the application as a
single command-line binary:

| Platform | Native package | Architectures |
|---|---|---|
| macOS | DMG containing `Robrix.app` | Apple Silicon, Intel |
| Debian/Ubuntu | DEB | arm64, x86-64 |
| Windows | NSIS installer | x86-64, including Windows on ARM emulation |

The release workflow builds those packages first. After every complete desktop
build, `scripts/generate-distribution-assets.sh` reads GitHub's SHA-256 digests
and generates:

- `robrix-installer.sh`
- `robrix-installer.ps1`
- `SHA256SUMS`
- `robrix-dist-manifest.json`
- the `robrix` npm installer package
- a Homebrew cask
- Winget multi-file manifests

The generated installers are version-pinned. The stable
`releases/latest/download` URLs select the newest stable installer script, but
that script still downloads the exact tag and verifies the exact digest embedded
when the release was built.

## Initial Bootstrap

The generated shell and PowerShell URLs do not exist on releases created before
this automation. Publish one complete desktop release after merging these files
to make those `releases/latest/download` commands live. Existing native packages
remain available from the repository's Releases page.

Homebrew becomes available when the generated cask is merged into the default
branch. npm additionally needs the first successful publish with `NPM_TOKEN`;
Winget needs an accepted manifest pull request in `microsoft/winget-pkgs`.

## Channel Automation

### Homebrew

The repository acts as its own tap. Stable release tags generate
`Casks/robrix.rb` and open a pull request containing the new version and both
macOS checksums. The pull request is deliberately not merged automatically.

No third-party Homebrew repository or credential is required.

### npm

Stable releases publish the unscoped `robrix` package to the `latest` npm
distribution tag. Prereleases publish to `next`, so a release candidate cannot
replace the default stable package.

Configure the repository secret `NPM_TOKEN` with permission to publish the
`robrix` package. Without it, the release workflow skips registry publication
and leaves the generated npm tarball attached to the GitHub release.

The npm package invokes the same native installer scripts. Set
`ROBRIX_NPM_SKIP_INSTALL=1` to install only the wrapper, then run
`robrix-install` later.

Removing the npm package removes only the wrapper command. Robrix itself is a
native application and must be uninstalled using the operating system's normal
application or package removal flow.

### Winget

Stable releases generate manifests for `ProjectRobiusChina.Robrix`. Configure
`WINGET_CREATE_GITHUB_TOKEN` with the permissions required by Microsoft's
WingetCreate tool. The workflow submits those manifests as a pull request to
`microsoft/winget-pkgs`.

Without that secret, the workflow skips submission and leaves a manifest ZIP on
the GitHub release. The initial Winget pull request must pass Microsoft's
installer and publisher validation before `winget install` becomes available.

## Local Validation

Run:

```sh
./scripts/test-distribution.sh
```

The test uses checked-in release metadata and does not download or install
Robrix. It checks shell syntax, all OS/architecture selections, the distribution
manifest, npm package contents, Homebrew Ruby syntax, Winget YAML, and unresolved
template placeholders.

Generated installers also support selection-only checks:

```sh
ROBRIX_INSTALLER_DRY_RUN=1 ./robrix-installer.sh
```

The macOS DMG contains the MIT license and normally presents it through the
terminal before mounting. For a noninteractive installation, review the license
first and set `ROBRIX_ACCEPT_LICENSE=1`.

For a new release, the Linux matrix gives Ubuntu 22.04 and Ubuntu 24.04 packages
distinct asset names. The shell installer selects Ubuntu 24.04 only on that
release and uses the Ubuntu 22.04 build as the baseline for other supported
Debian-family systems.
