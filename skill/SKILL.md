---
name: coord
description: Use when starting any coding task in a repository whose AGENTS.md contains a coord block (markers `<!-- coord:begin -->`) - coordinates parallel agent sessions by claiming files before editing, reordering work on conflicts, and making task-scoped commits
---

# Coordinating Parallel Edits with coord

Multiple agent sessions may be editing this repository right now. `coord` is
the shared ledger that keeps you from stomping each other. The protocol is in
AGENTS.md; this skill is how to use it well.

## Golden path

1. `coord task start "<one-line description>"` → remember the id (e.g. T12).
2. `coord claim -t T12 <paths>` **before** creating or editing anything.
3. Edit, test as usual. Claim more paths as your plan grows.
4. `coord commit -t T12 -m "<message>"` (never plain `git commit -a` / `git add -A`).
5. `coord task done T12` the moment the task is finished.

## Writing useful task descriptions

Other sessions decide how to reorder their work based on your description.
Write what a colleague would need: "refactor login flow (auth.ts, session.ts)"
beats "fix stuff". Mention the subsystem, not just the verb.

## When a claim is refused (exit code 2)

The output tells you who holds the path, for which task, since when. Do NOT
edit the file anyway. In order of preference:

1. **Reorder**: claim and work on your non-conflicting files first, then
   re-claim the contested path later — the holder may be done by then.
2. **Wait and retry**: if the contested file is your only remaining work,
   check `coord status` between your other steps.
3. **Co-edit deliberately**: only when the edits cannot interfere (e.g. you
   append to a changelog they touched), re-claim with `--force`. This is
   recorded and visible to both sessions.

Never work around a refusal by editing without a claim — that recreates the
exact conflict this protocol exists to prevent.

## Claim granularity

- Claim **files** for surgical changes; claim a **directory** when you will
  rework most of it (this blocks all files beneath it).
- Over-claiming starves other sessions; claim what you will actually touch.
- More claims later is cheap: `coord claim -t T12 <new-path>` any time.

## Stale tasks

`coord status` marks tasks whose session died as STALE. Their files are safe
to claim — you will see "taken over from stale T<n>" in the output. If you
crashed and restarted, start a fresh task; do not reuse the dead task's id.
