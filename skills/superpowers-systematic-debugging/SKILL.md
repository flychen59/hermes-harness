---
name: superpowers-systematic-debugging
description: "Use when something is broken, tests are failing, or behavior is unexpected. Follows a 4-phase root cause process instead of guessing."
tags: [debugging, troubleshooting, root-cause, fixing]
user-invocable: true
---

# Systematic Debugging

4-phase root cause process. No guessing, no random changes.

## Phase 1: Observe

- What is the actual behavior? (not what you expected)
- What is the exact error message?
- When did it last work? What changed?
- Can you reproduce it reliably?

## Phase 2: Hypothesize

Based on observations, form 2-3 hypotheses ranked by likelihood:
1. Most likely cause → what evidence supports it?
2. Second most likely → what evidence supports it?
3. Least likely but possible → what evidence?

## Phase 3: Test

For each hypothesis (most likely first):
- Design the smallest possible test that confirms or refutes it
- Run the test
- Record the result
- If refuted, move to next hypothesis
- If confirmed, move to Phase 4

**Never change code to "see if it fixes it"** — that's not debugging, that's hoping.

## Phase 4: Fix

Once root cause is confirmed:
- Write a failing test that demonstrates the bug
- Make the minimal fix
- Verify the test passes
- Run the full test suite
- Commit with a message that references the root cause

## Anti-Patterns

- ❌ "Let me just try changing this one thing"
- ❌ Adding print statements everywhere without a hypothesis
- ❌ Fixing symptoms instead of root causes
- ❌ Skipping the failing test because "it's obvious"

## Verification

After fixing:
- Does the original reproduction case pass?
- Do all existing tests still pass?
- Could this bug exist elsewhere in the codebase?
