---
description: Write to git, kin, progress.md, and mistakes.md before ending a session
agent: build
---

Write a handoff report before ending this session. Do the following:

1. Run `git log --oneline -10` and `git status` to capture recent changes.
2. Run `kin status` to capture the semantic graph state.
3. Read `.opencode/progress.md` and update it with the current state — what was done, what's pending, any new decisions or bugs discovered.
4. Read `mistakes.md` and append anything that went wrong during this session — failed approaches, wrong assumptions, things to avoid next time. If `mistakes.md` does not exist, create it.
5. Commit the updated files with an appropriate gitmoji-prefixed message.

Be concise. Focus on what changed and what the next person needs to know.
