# coord — Multi-Agent Parallel Edit Coordination — Design

Date: 2026-07-25
Status: direction confirmed with user; implementation plan pending

## Problem

When multiple coding agents (or multiple sessions of one agent) work on different tasks in the same repository concurrently:

1. They may edit the same file simultaneously, overwriting each other or producing conflicts.
2. They share one git index, so `git add -A` sweeps another session's half-finished dirty files into a commit.
3. There is no way to tell which file is currently being modified by whom, for what task.

Goal: make parallel edits naturally conflict-free without blocking any session's thinking or progress, with clear per-file task attribution.

## Core Philosophy

**Coordination by convention and cooperation, not interception and enforcement.**

- No filesystem watching, no enforced hook interception, no resident process (daemon).
- Installation injects the collaboration protocol into the repo's `AGENTS.md`; agents read the rules and **register before writing** (claim) and **deregister when finished** (done).
- The CLI is a shared task ledger: bookkeeper + information desk, not a gatekeeper.
- `AGENTS.md` is the cross-agent de facto standard (read by Claude Code / Codex / Cursor), so the protocol is agent-agnostic by nature; Claude Code additionally gets a skill with richer workflow guidance.

## Confirmed Key Decisions

| Decision point | Conclusion |
|---|---|
| Target ecosystem | Claude Code first; core is agent-agnostic (pure CLI protocol) |
| Conflict semantics | Lease + inform-and-reorder: the latecomer receives "who / which task / how long" intel and decides to reorder subtasks, wait, or co-edit via `--force` |
| Lease acquisition | Agent explicitly runs `claim` (driven by AGENTS.md rules), not implicit hook interception |
| Lease release | Explicit `task done`; crash scenarios covered by lazy cleanup |
| Commit strategy | Agent commits itself; CLI provides a scoped wrapper that stages only files claimed by that task |
| Resident process | None. State is a SQLite file inside the repo; short-lived CLI invocations with file locking for atomicity |
| Queue-and-merge staging | Explicitly rejected (pretending a write succeeded makes the agent reason from a false premise) |

## Architecture & Components

Three deliverables, one GitHub repo:

```
coord/
├── cli/          # Rust CLI (the ledger core)
├── skill/        # Claude Code skill (workflow guidance)
└── templates/    # AGENTS.md injection block template
```

### 1. CLI (Rust)

State store: `<repo>/.agentcoord/state.db` (SQLite), added to `.gitignore` by `coord init`. Every command is a short-lived process; SQLite's own locking guarantees concurrent atomicity.

Command surface (fixed at these six for v1 — YAGNI):

```
coord init                        # inject AGENTS.md block + create state db + .gitignore entry
coord task start "<description>"  # register task → returns task id (e.g. T12)
coord claim -t T12 <path>...      # register files/dirs before writing; if occupied, returns occupancy intel (not a hard refusal)
coord status                      # dashboard: which task holds which files, for how long, staleness
coord commit -t T12 -m "<msg>"    # stage only paths claimed by this task, commit with task info attached
coord task done T12               # release all registrations for the task, print a handoff summary
```

**Task identity propagation**: each agent shell invocation is an independent process, so task id cannot ride on environment variables or cwd — it must be passed explicitly (`-t`). The output of `task start` explicitly reminds the agent to "pass -t T12 on subsequent commands"; the AGENTS.md block and the skill reinforce this. A side benefit: one session can drive multiple tasks in parallel.

**PID liveness target**: at `task start`, walk up the ancestor process chain from the CLI itself and record the first long-lived ancestor (normally the agent process) as the liveness target; where probing is unavailable (odd process trees, containers), degrade to pure time-limit judgment.

Example `claim` conflict output (machine-readable + human-readable):

```
CLAIMED src/auth.ts
  by T12 "refactor login flow" (session 48291, 8 minutes ago)
Suggestion: work on other files first, or check progress via coord status;
if you judge the edits are truly parallel-safe, re-claim with --force to
register co-editing (leaves an audit trail).
```

`--force`: permits co-editing, recorded as a co-claim visible to both parties in `status` — at your own risk, on the record.

### 2. AGENTS.md Injection Block

`coord init` inserts a managed block into `AGENTS.md` at the repo root (created if absent):

```
<!-- coord:begin -->
(The collaboration protocol for agents: run `task start` when a task begins;
`claim` before modifying any file; on occupancy, reorder subtasks based on the
returned intel; finish with `coord commit` + `task done`.)
<!-- coord:end -->
```

- The marker block makes `coord init` idempotent (re-running only updates the block); `coord uninit` (deferred to v2) can remove it cleanly.
- The block stresses three iron rules: **claim before writing, commit only via coord commit, always end with done**.

### 3. Skill (Claude Code enhancement layer)

Skill content (a "use it well" layer, not an enforcement layer):

- How to write task descriptions other sessions can understand;
- How to reorder subtasks upon receiving occupancy intel (do the non-conflicting parts first);
- Recognizing zombie registrations and the safe takeover procedure;
- Claim-granularity guidance for multi-file tasks (file vs directory).

## Data Model (SQLite)

```
tasks(id, description, session_pid, started_at, finished_at NULL)
claims(task_id, path, claimed_at, released_at NULL, forced BOOL)
```

Path conflict rule: a new claim conflicts with any unreleased claim whose path has a containment relationship with it (file ≺ directory).

## Failure Handling

- **Zombie registrations**: a crashed session never runs `task done`. Every CLI invocation performs lazy cleanup: probe the PID of unfinished tasks; if the PID is dead or the registration exceeds the time limit (default 2 hours, configurable) → mark `stale`, show clearly in `status`, and tell subsequent claimers "the original holder is gone; safe to take over."
- **Hand-edited AGENTS.md**: only the marker block is ever touched; content outside it is never modified; if the block is deleted, `init` re-injects it.
- **No git repo**: `coord commit` errors with a hint; all other commands work (the ledger does not depend on git).

## Explicitly Out of Scope (v1)

- Filesystem watching / fswatch;
- Enforced hook interception (architecture-compatible; may become an opt-in safety net in v2);
- Resident daemon;
- Write staging queues with automatic merge;
- Cross-machine / remote coordination (local multi-session only).

## Success Criteria

1. Two Claude Code sessions run different tasks in the same repo concurrently with zero file stomping and zero commit cross-contamination.
2. At any moment, `coord status` explains every registered file's owning task and duration.
3. An agent with no skill installed, guided only by the AGENTS.md block, follows the protocol correctly (verify generality with Codex/another agent).
4. Single-session usage adds no burden (protocol overhead ≤ 3 CLI calls per task).

## Testing Strategy

- CLI core (claim conflict resolution, path containment, lazy cleanup, scoped commit): Rust unit + integration tests, TDD;
- Concurrency: contention tests with multiple processes claiming the same path simultaneously (exactly one must win);
- End-to-end: scripts simulating two sessions running the full protocol;
- Skill/AGENTS.md copy: manually verify compliance with real dual sessions.
