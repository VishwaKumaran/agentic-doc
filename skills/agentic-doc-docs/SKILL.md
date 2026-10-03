---
name: agentic-doc-docs
description: Completes and updates a project's documentation using agentic-doc (coverage,
  changes, relations). Use when the user asks to complete, catch up on, or update the
  documentation of the documented project.
disable-model-invocation: true
---

# agentic-doc-docs

This skill is a **client of the `agentic-doc` CLI**. It reads the index produced by
the tool, reads the source code and existing documentation itself, writes or enriches
Markdown pages, and registers relations via `relations add`.

It never fabricates JSON by hand, never edits `.agentic-doc/relations.json` directly,
and never runs `git`.

---

## Prerequisites

- `agentic-doc` is installed and available on the `PATH`.
  Install: `cargo install --git https://github.com/VishwaKumaran/agentic-doc`
- All commands are run **from the docs workspace** (the directory containing
  `.agentic-doc/config.json`).
- The docs workspace has been initialised with `agentic-doc setup <project-path>`.

---

## Modes

| Mode | Trigger | Description |
|---|---|---|
| `diff` | default | processes what changed since the last snapshot |
| `backlog` | explicit | catches up undocumented elements |

**Default budget:** 3 pages / 10 elements per run.

---

## Procedure (exact sequence)

### Preflight — required before any write

Run these three checks before writing anything. Stop immediately if any check fails.

```bash
# 1. Verify the tool is installed
agentic-doc --version
# If the command is not found: stop and tell the user to install agentic-doc
# with: cargo install --git https://github.com/VishwaKumaran/agentic-doc

# 2. Verify the relations sub-command is available
agentic-doc relations list --json
# If this fails: stop — "agentic-doc does not support relations yet"

# 3. Verify schema_version == 1
# Read schema_version from the JSON above. If != 1: stop and report the version
```

All three checks must pass before proceeding.

### Step 0 — Understand the workspace and project structure

**Run this step before Step 1. It is mandatory.**

The goal is to determine the correct directory where Markdown files must be written,
so that the agent never falls back to writing at the workspace root by default.

```bash
# 1. Read the workspace config to locate the project
cat .agentic-doc/config.json
# Fields used: project_path (path to the source project relative to the workspace)

# 2. List the top-level directories of the docs workspace
ls -1
# Look for: docs/, content/, pages/, src/, or any named directory containing .md files

# 3. Find where existing .md files live
find . -name "*.md" -not -path "./.agentic-doc/*" -not -path "./node_modules/*" | head -30
# Identify the common prefix directory (e.g. docs/, content/, fr/, en/, …)

# 4. List the top-level directories of the source project
ls -1 <project_path>
# Understand the source layout (src/, lib/, …) to anticipate identifier prefixes
```

From these four observations, derive **docs_dir**: the directory inside the workspace
where all Markdown pages live. Examples:

| Observation | docs_dir |
|---|---|
| All .md files are under `docs/` | `docs/` |
| All .md files are under `content/` (Nuxt/Content) | `content/` |
| All .md files are directly at the root (no sub-directory) | `.` (root) |
| All .md files are under `fr/` or `en/` | use the matching language sub-directory |

**Record `docs_dir` explicitly.** Every new file created in Step 6b MUST be placed
inside `docs_dir`, not at the workspace root unless `docs_dir` is `.`.

If the workspace has no existing Markdown files yet (brand-new workspace), examine
`nuxt.config.ts` / `package.json` to detect a Nuxt/Content site; otherwise default
`docs_dir` to `.`.

### Step 1 — Changed elements

```bash
agentic-doc status --json
```

Fields used: `changes[].element`, `changes[].kind`, `changes[].type`, `summary.changes`.

In `diff` mode: extract modified/added elements.
In `backlog` mode: run this step but do not use the result for the work list.

> **Never run `scan` here.** That would destroy the diff the skill must consume.

### Step 2 — Coverage BEFORE

```bash
agentic-doc docs --coverage --all --json
```

Fields used:
- `coverage.rate` — coverage rate before the run
- `coverage.undocumented` — count of undocumented elements
- `undocumented[].element`, `.file`, `.kind` — backlog work list
- `unreferenced_documents[]` — documents with no relation

Record this rate: it is the baseline for measuring progress.

### Step 3 — Relation state

```bash
agentic-doc relations list --json
agentic-doc relations check --json
```

Fields used:
- `relations[].resolution` — `ok` | `source_not_found` | `target_not_found` |
  `source_and_target_not_found`
- `summary.sources_not_found`, `summary.targets_not_found`, `summary.duplicates`

Report orphaned relations. Never delete a relation or a document.

### Step 4 — Build the work list

**`diff` mode:**

```bash
agentic-doc docs --json
```

Fields used: `impacts[].document`, `impacts[].reason`, `impacts[].confidence`.

Work list = impacted pages + modified elements not yet covered, within the budget.

> Note: in `docs --json`, `changes` is an integer (number of changes detected),
> not an array. The array of impacted documents is `impacts[]`.

**`backlog` mode:**

Work list = elements from `undocumented` (Step 2), capped at the budget
(10 elements / 3 pages). **The agent decides how to group elements into pages.**
Several related elements may share a single page when they cover the same topic;
the agent also decides the page's table of contents. Elements are sourced in
alphabetical order by file.

**Orphaned documents (`unreferenced_documents`):**
- If an obvious match with a known source element exists: declare the relation.
- Otherwise: report in the run summary; never merge documents, never delete anything.

> `README.md` at the root and navigation pages are expected in `unreferenced_documents`.
> This is not a defect to fix.

### Step 5 — Show the plan and wait for approval

Display:
1. Mode and budget.
2. List of pages to create or enrich, with associated elements.
3. Relations to declare.
4. Items flagged as orphaned or undocumentable.

**Wait explicitly for the user's approval before writing anything.**
There is no `--yes` flag in this version.

### Step 6 — Write (for each retained unit)

#### 6a. Read the source file in full

Read the entire source file. The tool output never contains line numbers: do not
mention or promise them.

#### 6b. Write or enrich the Markdown

Writing rules:
- **H1 required** (first line `# Title`): this is the only title the tool can read.
- **Mandatory minimum sections**: `Role`, `Usage`, `Key Elements`, `Examples`
  (or `Rôle`, `Utilisation`, `Éléments principaux`, `Exemples` in French).
  These sections may be extended.
- **Language**: that of the existing documents in the workspace; otherwise, the
  workspace language. Use `> ⚠️ Unverified` in English corpora,
  `> ⚠️ À vérifier` in French corpora, for anything not verified in the source.
- **Rewrite**: allowed only if existing content has become factually wrong.
- **Prohibited**: inventing a parameter, example, or use case; copying code beyond
  a short signature; removing human-authored content.

**Enrich an existing page first.** Create a new page only when no existing page
covers the topic.

**File destination (in priority order):**

> **The `docs_dir` derived in Step 0 is the authoritative destination.**
> Never write a file at the workspace root unless `docs_dir` is explicitly `.`.
> When in doubt, re-read the result of Step 0 before writing.

1. `docs_dir` from Step 0 — **always the primary rule**.
   - Place the new file directly inside `docs_dir/` (e.g. `docs/authentication.md`).
   - If existing pages in `docs_dir` are organised in sub-directories by topic,
     follow that convention (e.g. `docs/auth/authentication.md`).
2. Directory inferred from existing relations (where already-documented pages live) —
   use this to refine the sub-directory within `docs_dir`, never to override it.
3. Detected Nuxt/Content site (`nuxt.config.ts` + `package.json` + `content/`)
   → `docs_dir` is `content/`. Add front-matter (`title`, `description`) + H1.
4. If a template page exists in the corpus: follow its location; it overrides the
   sub-directory but not the `docs_dir` root.

**Forbidden directories:** `public/`, `.nuxt/`, `node_modules/`, `.agentic-doc/`.

**Front-matter format (detected site):**
```markdown
---
title: "Page title"
description: "Short description"
---

# Page title
```

#### 6c. Declare relations (same run as the write)

```bash
# File-level relation (required for every page written)
agentic-doc relations add "<file-identifier>" "<page.md>"

# Direct relation (only for symbols that have their own section in the page)
agentic-doc relations add "<symbol-identifier>" "<page.md>" --confidence high|medium|low
```

Rules:
- **Stateless:** the skill's state IS the output of the CLI commands. Always declare
  the relation in the SAME execution run as the written page, otherwise the work is
  invisible in the next run.
- **Identifiers are copied character-for-character** from `--json` output.
  Never infer an identifier from a symbol name in the code.
- **File-level relation is mandatory** for each page created or enriched.
- **Direct relation** only for symbols that have a dedicated section in the page
  (no "decorative" relation that would artificially inflate confidence).
- If `add` fails (exit code 1): record the failure, continue, never invent an
  alternative identifier. Report it in the summary.
- Create the Markdown file **before** calling `relations add`: the target must exist.

**Valid identifier forms:**

| Form | Element |
|---|---|
| `src/foo.py` | the file itself |
| `src/foo.py::MyClass` | a class |
| `src/foo.py::my_func` | a module-level function |
| `src/foo.py::MyClass.method` | a method |

No parentheses in identifiers. Variables and attributes are not elements.

### Step 7 — Post-write verification

```bash
# Measure progress (delta in coverage rate)
agentic-doc docs --coverage --json

# Verify no orphaned relations
agentic-doc relations check --json

# Verify no textual warnings
agentic-doc docs --coverage
```

### Step 8 — Final scan (conditional)

```bash
agentic-doc scan
```

**Run `scan` ONLY if no planned work remains incomplete.**

- "Undocumentable" (generated code, test file, empty file): an acknowledged finding
  that does **not** block the scan.
- "Incomplete" (budget exhausted, write failed, `add` refused): blocks the scan.
  The summary states what to resume next run.

---

## Run summary

The summary must be **quantified**, with before/after comparison:

```
## agentic-doc-docs run summary

**Mode:** diff | backlog
**Budget:** X pages / Y elements

### Coverage
- Before: XX% (N undocumented elements)
- After:  YY% (M undocumented elements)

### Pages written / enriched
| Page | Action | Associated elements |
|---|---|---|
| authentication.md | created | src/auth.py, src/auth.py::AuthService |

### Relations declared
| Source | Target | Result |
|---|---|---|
| src/auth.py | authentication.md | ok |

### Failures and reports
(list of refused `add` calls, orphaned relations, undocumentable elements)

### Final scan
- Executed: yes | no (reason)

### What remains
(unprocessed elements, budget exhausted, resume in backlog mode)

### Git suggestion
git add -A && git commit -m "docs: update via agentic-doc-docs"
```

On individual failure: continue and report. Never stop at the first refusal.

---

## Guardrails

- **Write only** to the destination determined by Step 6b.
- **Never** write to `public/`, `.nuxt/`, `node_modules/`, `.agentic-doc/`.
- **Never** edit `.agentic-doc/relations.json` by hand: use `relations add`.
- **Never** delete a document or an existing section; report them instead.
- **Never** invent an identifier: copy them from `--json` output.
- **Never** run `git`.
- **Never** promise a command or flag not in the CLI contract.
- **Never** run `scan` at the start of an execution.
- **No absolute paths, no `~`**: invoke `agentic-doc` from the `PATH`.
- **Preflight is mandatory** before any write.

---

## Pitfalls

- **Writing at the workspace root instead of `docs_dir`**: the most common mistake.
  Always run Step 0 and record `docs_dir` before writing. If existing pages live in
  `docs/`, every new page must also go into `docs/`, never at the workspace root.
- The `MarkdownAnalyzer` ignores names starting with `.`, and also `node_modules`,
  `target`, `dist`, `build`, `.agentic-doc`, `.git`. **`public/` is NOT ignored**:
  a `.md` placed there becomes a "document". Do not write there.
- `docs_root` is `.` in real workspaces: the entire workspace is scanned.
- The title read by the tool is the **first `# ` line**: not `##`, not a `title:`
  front-matter key alone.
- Line numbers do not exist in **any** tool output.
- Do not implement automatic relation discovery: no `relations discover` sub-command,
  no `candidate` status. Out of scope.
- `README.md` at the root and navigation pages are normally in `unreferenced_documents`.
  Do not "fix" them.

---

## Command reference

| Command | `cli.md` section | Use in this skill |
|---|---|---|
| `agentic-doc --version` | §3 | preflight: verify tool is installed |
| `agentic-doc relations list --json` | §8 | preflight: verify schema_version == 1 |
| `agentic-doc status --json` | §6 | step 1: changes since last snapshot |
| `agentic-doc docs --coverage --all --json` | §7 | step 2: coverage BEFORE |
| `agentic-doc relations list --json` | §8 | step 3: declared relations |
| `agentic-doc relations check --json` | §8 | step 3 & 7: relation audit |
| `agentic-doc docs --json` | §7 | step 4 (diff mode): impacts |
| `agentic-doc docs --coverage --json` | §7 | step 7: coverage AFTER |
| `agentic-doc docs --coverage` | §7 | step 7: textual warnings check |
| `agentic-doc relations add <source> <target> [--confidence]` | §8 | step 6c: declare a relation |
| `agentic-doc scan` | §5 | step 8: final snapshot (conditional) |
