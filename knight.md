# Knight Protocol — Codex Agent Dispatch

Claude instances can command Codex agents as autonomous workers for parallel, multi-file refactoring and implementation tasks.

## Invocation

```bash
cat <<'PROMPT' | codex exec --full-auto -C /path/to/repo 2>&1
[prompt]
PROMPT
```

From Claude Code, run with `run_in_background: true` on the Bash tool. This returns a task ID — check results later with `TaskOutput`. Launch multiple agents in a single message for parallel execution.

## Prompt Structure

State the goal and provide context. Codex with xhigh reasoning is a competent engineer — talk to it like one.

**Scope** — tell the agent what territory it owns. This can be folders, file patterns, or conceptual boundaries:

```
You own `src/codegen/contracts/schema-*/**`.
Other agents are working in `src/codegen/generator-core/` concurrently — don't modify anything there.
```

For parallel dispatch, the key rule is: **don't overlap writes**. Agents can read anything, but concurrent write targets must not collide. Scope by folder or by concern — whatever makes the boundary clear.

**Task** — describe what to accomplish. Paste type signatures, before/after examples, or reference patterns when they'd help orient the agent. Don't over-explain what it can infer from the code.

**Verification** — tell it what to run when done:

```
Run `pnpm typecheck` when finished.
```

## Session Continuity

Codex retains context between sessions. When dispatching a new task that builds on previous work, select a prior Codex session with relevant context. This gives the agent memory of the codebase's evolution without re-explaining the full history.

## Parallel Dispatch

Assign non-overlapping scopes to multiple agents and launch all in one message. The agents run concurrently and report back independently.

Scope strategies:
- **By folder**: Agent A owns `contracts/schema-*`, Agent B owns `generator-core/algebra/`
- **By concern**: Agent A handles type changes, Agent B handles consumer updates
- **By file list**: when folders don't map cleanly to the work

The only hard rule is no concurrent writes to the same file.

## Config

`~/.codex/config.toml`:

```toml
model = "gpt-5.3-codex"
model_reasoning_effort = "xhigh"
```

## Trust Calibration

xhigh reasoning is genuinely capable. State the goal, provide context, and let it work. It will figure out imports, unused symbol cleanup, and idiomatic patterns on its own.
