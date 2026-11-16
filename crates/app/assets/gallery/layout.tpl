# Pane layout — apply with right-click → Open layout,
# or `gsh orchestrate dev.ckp` from a tab.
# autorun: worktree   # apply automatically on new worktrees
panes:
  - name: Shell
    cwd: .
  - name: Agent
    cwd: .
    split: right
    command: claude
