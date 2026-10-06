# Page templates and quality checklists

These templates define the required shape and completion checks for pages written by
`agentic-doc-docs`. They are not a workflow: use the skill procedure to select an
approved page type and to verify the source before writing.

## Shared rules

- Start every file with its H1. Add front matter only when the corpus already uses it.
- Write in the corpus language. Mark unsupported claims with `> ⚠️ Unverified` or
  `> ⚠️ À vérifier`.
- Put explanatory prose before every code block. Do not invent examples, parameters, or
  use cases; beyond a short signature, quote code only when it exists in the source.
- Link only to pages that already exist in `docs_dir`. Omit a link section when no valid
  target exists and record that omission in the checklist.
- Preserve existing page content: enrich by appending unless a fact is wrong and the
  approved plan explicitly authorizes a restructuring.

## `reference`

Use for a symbol-driven page. Keep this template unchanged, including its existing header
shape; it does not require a positioning lead.

### Required sections

- `## Role` / `## Rôle` — 2–4 prose sentences that state the responsibility; no bullets.
- `## Usage` / `## Utilisation` — a step-by-step workflow in prose, then a code example.
- `## Key Elements` / `## Éléments principaux` — for every documented parameter,
  attribute, or method: name, role, and concrete value or use.
- `## Examples` / `## Exemples` — at least two examples, one minimal and one realistic;
  introduce each with prose.
- `## Errors and edge cases` / `## Erreurs et cas limites` — optional, when source
  material treats them.

### Checklist

- [ ] The four required sections are present in the corpus language.
- [ ] Role has 2–4 prose sentences and no bullets.
- [ ] Usage explains the workflow before code.
- [ ] Key Elements explains each documented item beyond its signature.
- [ ] Examples include minimal and contextual cases, each introduced with prose.
- [ ] Errors and edge cases are covered when the source treats them.

## `overview`

Use as the entry point for a subsystem.

### Required sections

Immediately after the H1, write an untitled positioning lead of 1–3 sentences: what it is,
who it serves, and the problem it solves.

- `## What it does` / `## Ce qu'il fait` — prose plus capability bullets; bullets may link
  to existing pages.
- `## Getting started` / `## Premiers pas` — the smallest useful use, prose first, then one
  minimal code block.
- `## Make it yours` / `## Personnalisation` — extension or configuration; omit only when
  the source offers neither.
- `## Explore` / `## Explorer` — a page-to-one-line-description table for nearby corpus
  pages; omit when no target exists.
- `## Where it fits` / `## Sa place dans l'ensemble` — prose describing relationships with
  other system components.
- `## Where to go next` / `## Pour aller plus loin` — two or three typed reader paths
  (deepen, use, or understand architecture); omit when no target exists.

### Checklist

- [ ] A 1–3 sentence positioning lead follows the H1.
- [ ] What it does combines explanatory prose with grounded capabilities.
- [ ] Getting started has prose before exactly one minimal code block.
- [ ] Make it yours is present, or its absence is justified by the source.
- [ ] Explore and Where to go next contain only existing targets, or each omission is noted.
- [ ] Where it fits explicitly explains the component's system relationships.

## `concept`

Use for a cross-cutting concern or design intention.

### Required sections

Immediately after the H1, write a positioning lead of 1–3 sentences.

- `## Purpose` / `## Objectif` — 2–4 sentences explaining why this concept exists and the
  project-specific problem it solves.
- Two to five agent-named theme groups — each H2 has two to four short assertions: an
  imperative bold title followed by 1–3 prose sentences. Code is not required.
- `## Applying it here` / `## L'appliquer ici` — how the concept appears in real code; use
  real identifiers only and prose before optional short code.
- `## Related pages` / `## Pour approfondir` — links to existing pages; omit when no target
  exists.

### Checklist

- [ ] A positioning lead follows the H1.
- [ ] Purpose is 2–4 project-specific prose sentences.
- [ ] There are 2–5 named theme groups, each with 2–4 imperative bold assertions.
- [ ] Applying it here uses only verified identifiers and explains their application.
- [ ] Related pages has no dead link, or its omission is noted.
- [ ] The tone expresses durable principles rather than a code dump.

## `tutorial`

Use for a sequential procedure.

### Required sections

Immediately after the H1, write a 1–2 sentence lead stating what the reader will be able to
do by the end.

- `## Before you start` / `## Avant de commencer` — prerequisites: files, configuration, or
  prior concepts; omit only when everything is supplied.
- At least three ordered `## Step N — …` / `## Étape N — …` sections — each explains the
  action before an optional single code block and builds on the prior step.
- `## Recap` / `## Récapitulatif` — prose describing what was built or learned.
- `## Next steps` / `## Étapes suivantes` — links to existing continuation pages; omit when
  no target exists.

### Checklist

- [ ] A 1–2 sentence outcome lead follows the H1.
- [ ] Before you start is present, or its absence is justified because all prerequisites are supplied.
- [ ] At least three ordered steps build on one another.
- [ ] Each step explains before any optional, single code block.
- [ ] Recap states what the reader built or learned.
- [ ] Next steps has no dead link, or its omission is noted.

## Common completion checklist

- [ ] H1 is the first line.
- [ ] The type matches the approved plan.
- [ ] Prose precedes every code block.
- [ ] No link target is missing.
- [ ] No existing content was removed.
- [ ] The corpus language and uncertainty marker are correct.
- [ ] No CLI command is invented.
- [ ] Every identifier was copied from `--json` output or read in source.
