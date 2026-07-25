## Parallel-edit coordination (coord)

This repository uses `coord` so that multiple coding agents can work in
parallel without conflicts. Three iron rules:

1. **Claim before writing.** Never create or modify a file you have not claimed.
2. **Commit only via `coord commit`.** It stages only your task's files, so you
   can never sweep another session's half-finished work into your commit.
3. **Always finish with `coord task done`.** It releases your claims for others.

Workflow:

1. When you begin a task, run:
   `coord task start "<one-line task description>"`
   Note the returned task id (e.g. `T12`) and pass it via `-t` on every later command.
2. Before creating or editing any file, register it:
   `coord claim -t T12 <path>...`   (directories are allowed)
3. If a path is occupied, the output tells you who holds it and for which task.
   Do not modify that file. Reorder your work: do the non-conflicting parts
   first and re-claim later. Only if you are confident the edits cannot
   interfere, re-run with `--force` to register co-editing (leaves an audit trail).
4. Commit your work with: `coord commit -t T12 -m "<message>"`
5. When the task is complete, run: `coord task done T12`

Run `coord status` any time to see who is editing what. Tasks whose session
died are marked STALE — their files are safe to take over.
