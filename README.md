# coord

A shared task ledger that lets multiple coding agents (or multiple sessions
of one agent) edit the same repository in parallel — without stomping each
other, and with clear per-file attribution.

No daemon. No file watching. No hooks. Coordination is a *convention*:
agents read the protocol from `AGENTS.md` and voluntarily register what they
are about to edit. The CLI is a bookkeeper, not a gatekeeper.

## How it works

- `coord init` injects a managed protocol block into your repo's `AGENTS.md`
  (read by Claude Code, Codex, Cursor, …) and creates a SQLite ledger at
  `.agentcoord/state.db` (gitignored).
- Each agent session registers a task, **claims** paths before editing them,
  and releases everything when done.
- A claim on an occupied path is refused with intel — who holds it, for which
  task, since when — so the agent reorders its own work instead of blocking.
- `coord commit` stages and commits **only the task's claimed paths**
  (a git pathspec commit), so one session can never sweep another session's
  half-finished files into its commit.
- Crashed sessions leave no permanent locks: their tasks go **stale**
  (dead pid or age past `COORD_STALE_SECS`, default 2 h) and their files are
  safe to take over.

## Install

### Prebuilt binaries (no Rust required)

Download the archive for your platform (macOS arm64/x64, Linux x64/arm64)
from the [latest release](https://github.com/awesome-team-w/coord/releases/latest),
then:

```sh
tar -xzf coord-*.tar.gz
sudo mv coord /usr/local/bin/
```

### From source

```sh
cargo install --path cli
```

### Set up your repo

```sh
cd your-repo && coord init
```

Optional, for Claude Code: copy `skill/` into your skills directory as
`coord/` to give sessions richer workflow guidance.

## Commands

| Command | Purpose |
|---|---|
| `coord init` | Inject AGENTS.md block, create ledger |
| `coord task start "<desc>"` | Register a task, get its id |
| `coord claim -t T12 <path>...` | Register paths before editing (`--force` = co-edit) |
| `coord status` | Who is editing what, staleness flags |
| `coord commit -t T12 -m "<msg>"` | Commit only the task's claimed paths |
| `coord task done T12` | Release all claims, close the task |

Exit codes: `0` ok · `1` error · `2` claim refused (path occupied).

## Example session

```text
$ coord task start "refactor login flow"
Started T12: refactor login flow
Pass `-t T12` on every subsequent coord command for this task.

$ coord claim -t T12 src/auth.ts
registered src/auth.ts

# meanwhile, another session:
$ coord claim -t T15 src/auth.ts
CLAIMED src/auth.ts
  by T12 "refactor login flow" (session 48291, claimed 8 minutes ago)

1 path(s) occupied. Work on other files first, or check `coord status`;
if the edits are truly parallel-safe, re-run with --force to register co-editing.
```

## Design

See `docs/superpowers/specs/2026-07-25-coord-design.md`. v1 deliberately
excludes daemons, file watching, hook enforcement, and cross-machine
coordination — the rationale is in the spec.

## License

MIT — see [LICENSE](LICENSE).
