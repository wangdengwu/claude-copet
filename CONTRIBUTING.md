# Contributing — claude-copet

## Git conventions

### Committer identity

All commits in this repo use:

```
name:  dengwu.wang
email: me@dengwu.wang
```

This is set as **repo-local** git config (`.git/config`), so it applies automatically
in this project regardless of your global git identity:

```bash
git config user.name  "dengwu.wang"
git config user.email "me@dengwu.wang"
```

Do **not** commit with `dengwu.wang@sayweee.com` (the work address) in this project.
If a commit slips through with the wrong email, rewrite it before pushing:

```bash
FILTER_BRANCH_SQUELCH_WARNING=1 git filter-branch -f --env-filter '
export GIT_AUTHOR_NAME="dengwu.wang";  export GIT_AUTHOR_EMAIL="me@dengwu.wang"
export GIT_COMMITTER_NAME="dengwu.wang"; export GIT_COMMITTER_EMAIL="me@dengwu.wang"
' -- --all
```

### Commit messages

- Conventional-commit prefixes (`feat:`, `fix:`, `chore:`, `docs:`, …).
- AI-assisted commits end with a `Co-Authored-By:` trailer.

## Workflow

This project is planned as vertical slices. The source of truth is:

- **PRD:** `docs/prds/2026-06-25-claude-code-desktop-pet.md`
- **Tasks:** `tasks/2026-06-25-claude-code-desktop-pet/` (one file per slice, ordered by `blocked_by`)

Pick up a slice with `weee:dev` and build it test-first against the seams the PRD names.
