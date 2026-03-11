# dsync

A Rust CLI for syncing Markdown documents with external providers.

Initial providers:

- Google Docs
- Linear Docs

The goal is to keep `.md` as the local source of truth and make cross-linking between platforms easier.

## Overview

`dsync` supports:

- sync a local `.md` file to Google Docs and Linear, creating documents when they do not exist yet
- import a Google Doc into `.md`
- import a Linear Doc into `.md`
- keep cross-links automatically updated:
  - Google Docs <-> Linear
  - plus the file link in Git when the file is inside a git repository

## Installation

### Option 1: one-line install

```bash
curl -fsSL https://raw.githubusercontent.com/feliperbroering/dsync/main/install.sh | bash
```

The installer tries to download the latest matching GitHub release for your platform first. If that asset is not available yet, it falls back to building from source with Cargo.

Useful overrides:

```bash
curl -fsSL https://raw.githubusercontent.com/feliperbroering/dsync/main/install.sh | bash -s -- --version v0.1.0
curl -fsSL https://raw.githubusercontent.com/feliperbroering/dsync/main/install.sh | DSYNC_INSTALL_DIR="$HOME/bin" bash
```

### Option 2: local build (with Rust)

```bash
git clone https://github.com/feliperbroering/dsync.git
cd dsync
cargo install --path .
```

This makes the `dsync` binary available in the Cargo `PATH`.

### Option 3: use a prebuilt binary

Once releases are published, download the executable and place it in your `PATH`.

## Releases

GitHub Actions now validates prebuilt release archives for:

- `x86_64-unknown-linux-gnu`
- `aarch64-unknown-linux-gnu`
- `x86_64-apple-darwin`
- `aarch64-apple-darwin`

Versioning and GitHub Release orchestration are handled with `release-please`.

## Configuration

### Linear

Set the API token:

```bash
export LINEAR_API_KEY="lin_api_..."
```

### Google Docs

The CLI currently expects a pre-issued bearer token:

```bash
export GOOGLE_ACCESS_TOKEN="ya29...."
```

There is also a template file:

```bash
cp .env.example .env
```

Expected minimum token scopes:

- `https://www.googleapis.com/auth/documents`
- `https://www.googleapis.com/auth/drive`

> Next project step: a native OAuth flow inside the CLI.

## Usage

### 1) Sync local file -> providers (with automatic creation)

```bash
dsync ~/test/doc.md --gdoc --linear
```

Behavior:

- if `gdocUrl` does not exist in frontmatter and `--gdoc` is used, it creates a Google Doc
- if `linearDocId` or `linearDocUrl` does not exist and `--linear` is used, it creates a Linear Doc
- prompts interactively for:
  - Drive folder ID (optional)
  - Linear team/project

### 2) Import Google Docs into `.md` in the current directory

```bash
dsync --gdoc <GDOC_ID>
```

### 3) Import a Linear Doc into `.md` in the current directory

```bash
dsync --linear <LINEAR_DOC_ID>
```

## Frontmatter

Expected resulting example:

```yaml
---
gdocUrl: "https://docs.google.com/document/d/<DOC_ID>/edit"
linearDocUrl: "https://linear.app/<workspace>/document/<DOC_ID>/<slug>"
linearDocId: "<DOC_ID>"
gitUrl: "https://github.com/<org>/<repo>/blob/<branch>/docs/file.md"
---
```

## Architecture (extensible)

The project is structured to support additional providers easily:

- Notion (future)
- Evernote (future)
- other repositories and document stores

Evolution strategy:

1. abstract the provider interface
2. implement read/write support for each provider
3. keep cross-links and frontmatter as the stable contract

Short-term roadmap:

- [ ] Google provider with native OAuth (no manual token)
- [ ] Provider Notion
- [ ] Provider Evernote

## Security

- do not commit tokens to the repository
- use environment variables
- prefer tokens with the smallest possible scope

## License

MIT
