# agentic-doc

`agentic-doc` is a command-line tool that keeps a software project and its documentation
in sync. It analyses Python source code, tracks changes between snapshots, and reports
which documentation pages are potentially outdated.

> **PyPI name notice:** the name `agentic-doc` on PyPI is already taken by
> [Landing AI's `agentic-doc`](https://pypi.org/project/agentic-doc/) — a completely
> different product. Never install `agentic-doc` via `uv tool install agentic-doc` or
> `pip install agentic-doc`. The Python distribution of this tool will be published as
> `agentic-doc-cli` (coming soon).

---

## Installation

**Per-user (Rust, any OS):**

```bash
cargo install --git https://github.com/VishwaKumaran/agentic-doc
```

This puts `agentic-doc` in `~/.cargo/bin`, which is on **your** PATH only.

**System-wide (macOS, Linux, Windows) — every user on the machine:**

Builds from a checkout of this repository (nothing is downloaded, no release is
required) and installs the binary into a shared location that every user finds
on their PATH. Requires a Rust toolchain ([rustup.rs](https://rustup.rs)).

```bash
# macOS / Linux
git clone https://github.com/VishwaKumaran/agentic-doc
cd agentic-doc
sudo ./install.sh
```

```powershell
# Windows — from an elevated PowerShell
git clone https://github.com/VishwaKumaran/agentic-doc
cd agentic-doc
.\install.ps1
```

| Option | Effect |
|---|---|
| `--prefix DIR` / `-Prefix DIR` | installation prefix — default `/usr/local` on Unix (binary in `/usr/local/bin`), `%ProgramFiles%\agentic-doc` on Windows |
| `AGENTIC_DOC_PREFIX` | same as `--prefix`; the command line wins |

Re-running the installer **updates** the binary in place — there is no separate
uninstall step. On Unix, `--prefix` also makes the installer testable without
root, e.g. `./install.sh --prefix "$(mktemp -d)"`; that is what
`tests/install_smoke.sh` does:

```bash
sh tests/install_smoke.sh
```

**Install the agent skill (available now):**

```bash
npx skills add VishwaKumaran/agentic-doc
```

**Coming soon:**

```bash
# npm (coming soon — not yet available) — the CLI itself
# uv / PyPI (coming soon — not yet available)
# uv tool install agentic-doc-cli
```

---

## How it works

`agentic-doc` introduces a **docs workspace**: a dedicated directory that mirrors your
project and holds documentation, relations, and snapshots.

```
my-project-docs/          ← docs workspace (run all commands from here)
├── authentication.md
├── users.md
└── .agentic-doc/
    ├── config.json       ← links this workspace to the project
    ├── relations.json    ← explicit source → document relations
    └── snapshots/        ← project state snapshots
```

A **relation** links a source element (a file, class, function, or method) to a
documentation page. Relations make it possible to know which pages to review when
a specific symbol changes.

---

## Commands

### 1. `setup`

Initialises the docs workspace and links it to the project:

```bash
cd my-project-docs
agentic-doc setup ../my-project
```

### 2. `scan`

Analyses the project and records the current state as a reference snapshot:

```bash
agentic-doc scan
```

### 3. `status`

Shows what has changed in the project since the last snapshot:

```bash
agentic-doc status
agentic-doc status --json        # machine-readable output
```

### 4. `docs`

Reports which documentation pages are potentially affected by recent changes, or
shows overall documentation coverage:

```bash
agentic-doc docs                        # impact analysis
agentic-doc docs --coverage             # coverage report
agentic-doc docs --coverage --all       # full coverage (documented + undocumented)
agentic-doc docs --json                 # machine-readable impact output
agentic-doc docs --coverage --json      # machine-readable coverage output
```

### 5. `relations`

Manages explicit links between source elements and documentation pages:

```bash
# List all declared relations
agentic-doc relations list

# Declare a new relation (identifiers come from --json output, never guessed)
agentic-doc relations add "src/auth.py::AuthService" "authentication.md" --confidence high

# Remove a relation
agentic-doc relations remove "src/auth.py::AuthService" "authentication.md"

# Audit all relations (check for orphans)
agentic-doc relations check
agentic-doc relations check --json
```

### 6. `skills`

Installs the agent skill by delegating to the standard `npx skills` CLI. It is a thin
**alias** — a convenience so the install command lives next to the tool (see
[Agent skill](#agent-skill)):

```bash
agentic-doc skills install              # → npx -y skills add VishwaKumaran/agentic-doc
agentic-doc skills add                  # alias of install
agentic-doc skills install --agent claude-code
```

All arguments after `install`/`add` are passed through to `npx skills add`, so agent names
and flags are those of [`npx skills`](https://github.com/vercel-labs/skills) (`claude-code`,
`--global`, `--copy`, `--all`, …). This requires Node.js and network access.

---

## Complete example

```bash
# 1. Create and enter the docs workspace
mkdir my-project-docs && cd my-project-docs

# 2. Link the workspace to the project
agentic-doc setup ../my-project

# 3. Take an initial snapshot
agentic-doc scan

# --- work on the project ---

# 4. See what changed
agentic-doc status

# 5. See which docs to update
agentic-doc docs

# 6. Update authentication.md, then declare the relation
agentic-doc relations add "src/auth.py::AuthService" "authentication.md" --confidence high

# 7. Record the new baseline
agentic-doc scan
```

**Getting source element identifiers:**

```bash
# All identifiers, grouped by change type
agentic-doc status --json

# All identifiers, grouped by documentation status
agentic-doc docs --coverage --all --json
```

Identifier forms:

| Form | Element |
|---|---|
| `src/auth.py` | the file itself |
| `src/auth.py::AuthService` | a class |
| `src/auth.py::refresh_token` | a module-level function |
| `src/auth.py::AuthService.verify` | a method |

---

## Agent skill

This repository ships an agent skill that automates documentation coverage using the
CLI above. Agents (Codex, Cline, …) can use it to complete and update documentation
automatically.

**Install the skill:**

Use the standard skills CLI (requires Node.js and network access):

```bash
npx skills add VishwaKumaran/agentic-doc
```

The same install is available as a convenience alias on the tool:

```bash
agentic-doc skills install        # → npx -y skills add VishwaKumaran/agentic-doc
agentic-doc skills add            # alias of install
```

Every argument is passed through to `npx skills add`, so agent names and flags are those of
[`npx skills`](https://github.com/vercel-labs/skills) (for example `--agent claude-code`,
`--global`, `--all`). `npx skills` collects anonymous telemetry; disable it with
`DISABLE_TELEMETRY=1` or `DO_NOT_TRACK=1`.

The skill is located at `skills/agentic-doc-docs/SKILL.md`. It:

- Runs a preflight check (`agentic-doc --version`, `schema_version == 1`).
- Supports two modes: `diff` (what changed) and `backlog` (what is undocumented).
- Displays its plan and waits for user approval before writing anything.
- Declares relations in the same run as the corresponding page.
- Produces a quantified before/after summary.

---

## License

MIT — see [LICENSE](LICENSE).
