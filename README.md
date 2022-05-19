# Vers

A command line utility designed to help manage other dev tools on your machine, installing from GitHub Releases (more to come in the future).

## Getting Started

If you already have Rust installed the quickest install method is cargo install vers for now.
If not download the latest release for your system from GitHub Releases.

Setup your shell environment by using the following

```shell
# Shell can also be fish or zsh
$ vers env --shell bash >> ~/.bash_profile
export PATH="/Users/reynn/Library/Application Support/dev.reynn.vers/envs/global:$PATH"
```

Install your first tool is as easy as issuing something like the following

```shell
# This will install the latest version of the GitHub CLI and alias it so you can call it using `gh`
# if no alias is provided the repository name is used
$ vers add cli/cli --alias gh
--> Installing tool cli/cli@2.10.1

# You can use the --show (short: -S) flag to show a list of versions available and allows you to select one
$ vers add cli/cli --alias --show
--> Installing tool cli/cli@2.9.0
```

Vers has a method to auto detect the asset from the release that is appropriate for your system, sometimes this fails due since there is no standardization in file naming. To override the autodetect feature use the `--pattern (short: -p)` flag

```shell
# --pattern is a regular expression
$ vers add cli/cli --alias gh --pattern '.+macos.+'
```

The last important thing for the add feature is using a `--filter (short: -f)` this is used to search for the resulting binary once the asset down is complete and extracted, or not. This defaults to the alias if provided or the repository name.

```shell

```

## Environments

Multiple environments are supported but are still in an early state.

```shell
# the --pre-release (short: -P) flag can be used to include pre release versions in the results
$ vers --env betas add cli/cli --alias gh-beta --pre-release
```

## Directory Structure

```text
Permissions Size User  Date Modified    Name
 /Users/reynn/Library/Application Support/dev.reynn.vers
├──  envs
│  ├──  global
│  │  └──  # symlinks here to the correct version of the tool under ../tools
│  └──  global.json
└──  tools
   ├──  bitwarden
   │  └──  cli
   │     └──  1.22.1
   ├──  bootandy
   │  └──  dust
   │     └──  0.8.1-alpha.2
   ├──  BurntSushi
   │  └──  RipGrep
   │     └──  13.0.0
   ├──  charmbracelet
   │  └──  glow
   │     └──  1.4.1
   ├──  chmln
   │  └──  sd
   │     └──  0.7.6
   ├──  ClementTsang
   │  └──  bottom
   │     ├──  0.6.8
   │     └──  nightly
   ├──  cli
   │  └──  cli
   │     ├──  2.9.0
   │     ├──  2.10.0
   │     └──  2.10.1
   ├──  dalance
   │  └──  procs
   │     └──  0.12.2
   ├──  denisidoro
   │  └──  navi
   │     └──  2.19.0
   ├──  derailed
   │  └──  k9s
   │     └──  0.25.18
   ├──  digitalocean
   │  └──  doctl
   │     └──  1.75.0
   ├──  getzola
   │  └──  zola
   │     └──  0.15.3
   ├──  gohugoio
   │  └──  hugo
   │     └──  0.98.0
   ├──  helix-editor
   │  └──  helix
   │     └──  22.03
   ├──  jesseduffield
   │  └──  lazygit
   │     ├──  0.34
   ├──  junegunn
   │  └──  fzf
   │     └──  0.30.0
   ├──  k0sproject
   │  └──  k0sctl
   │     └──  0.13.0-rc.2
   ├──  koalaman
   │  └──  shellcheck
   │     └──  0.8.0
   ├──  kubernetes-sigs
   │  └──  kustomize
   │     └──  kustomize
   ├──  lotabout
   │  └──  skim
   │     └──  0.9.4
   ├──  neovim
   │  └──  neovim
   │     ├──  0.7.0
   │     └──  nightly
   ├──  nushell
   │  └──  nushell
   │     └──  0.62.0
   ├──  ogham
   │  └──  exa
   │     └──  0.10.1
   ├──  orhun
   │  └──  git-cliff
   │     └──  0.7.0
   ├──  sharkdp
   │  ├──  bat
   │  │  └──  0.21.0
   │  ├──  fd
   │  │  └──  8.3.2
   │  ├──  hexyl
   │  │  └──  0.9.0
   │  └──  hyperfine
   │     └──  1.14.0
   ├──  TomWright
   │  └──  dasel
   │     └──  1.24.3
   ├──  uutils
   │  └──  coreutils
   │     └──  0.0.13
   ├──  XAMPPRocky
   │  └──  tokei
   │     └──  12.1.2
   └──  zellij-org
      └──  zellij
         └──  0.29.1
```
