---
name: superpowers-subagent-driven
description: "Execute implementation plans by dispatching fresh subagent per task, with two-stage review after each: spec compliance review first, then code quality review."
tags: [execution, subagent, implementation, review]
user-invocable: true
---

# Subagent-Driven Development

Execute plan by dispatching fresh subagent per task, with two-stage review after each: spec compliance first, then code quality.

**Why subagents:** You delegate tasks to specialized agents with isolated context. By precisely crafting their instructions, you ensure they stay focused. They should never inherit your session's context or history — you construct exactly what they need. This also preserves your own context for coordination work.

**Core principle:** Fresh subagent per task + two-stage review (spec then quality) = high quality, fast iteration

**Continuous execution:** Do not pause between tasks. Execute all tasks from the plan without stopping. The only reasons to stop are: BLOCKED status you cannot resolve, ambiguity that genuinely prevents progress, or all tasks complete.

## The Process

### Per Task:

1. **Dispatch implementer subagent** with:
   - The task spec (full text from the plan)
   - Exactly which files to create/modify
   - The test to write first (TDD)
   - No inherited context from previous tasks

2. **Dispatch spec reviewer subagent** to verify:
   - Code matches the task spec exactly
   - All acceptance criteria met
   - No missing edge cases from spec

3. **Dispatch code quality reviewer subagent** to verify:
   - Code follows project conventions
   - No anti-patterns
   - Proper error handling
   - Clean, readable code

4. **Mark task complete** and move to next task

## Model Selection

Use the least powerful model that can handle each role:
- **Mechanical tasks** (isolated functions, clear specs): fast, cheap model
- **Integration/judgment tasks** (multi-file coordination, patterns): stronger model
- **Reviews**: medium model with careful instructions

## After All Tasks

1. Dispatch a final code reviewer for the entire implementation
2. Verify all tests pass end-to-end
3. Present summary with all commits made
