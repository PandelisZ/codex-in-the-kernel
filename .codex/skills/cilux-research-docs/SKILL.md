---
name: cilux-research-docs
description: Summarize Cilux experiments and findings into the docs website. Use when publishing or updating research notes, experiment summaries, validated findings, active tracks, or the /research page for this repository.
---

# Cilux Research Docs

Use this skill when the task is to turn repo evidence or live experiment output into a grounded update for the docs site.

## Primary Targets

- `docs/src/research-content.ts`
- `docs/src/research.ts`
- `docs/src/content.ts`

## Preferred Workflow

1. Gather evidence first

- Read the current `README.md`, relevant notes under `cilux/docs/`, and any cited test output or logs before writing claims.
- Prefer repo-backed evidence over memory or generic summary language.

2. Update the research page first

- Add or revise entries in `docs/src/research-content.ts`.
- Keep each track or entry explicit about its status:
  - `validated`
  - `active`
  - `blocked`
  - `planned`
- Each research entry should explain:
  - what changed
  - what evidence supports it
  - what it implies for the harness
  - what question remains next

3. Keep the homepage concise

- Only promote stable claims into `docs/src/content.ts` when the result is already well supported.
- The homepage is the high-level thesis and current state.
- `/research/` is the running log and synthesis surface.

4. Preserve the editorial bar

- Separate proven findings from hypotheses.
- Prefer exact wording over dramatic wording.
- Call out limitations directly when the evidence is incomplete.

5. Verify

- Run `pnpm run docs:build` after edits.
- If the update changes the public story materially, check that homepage links to `/research/` still make sense.

## Authoring Rules

- Extend an existing research track before creating a new category.
- Use additive updates instead of rewriting history unless the older claim is wrong.
- Do not invent dates, benchmarks, or kernel behavior that the repo does not support.
- When an experiment is still inconclusive, publish it as `active` or `blocked`, not `validated`.

## Promotion Heuristic

Promote a research result from `/research/` to the homepage only when:

- the repo currently demonstrates it
- the claim survives concise wording
- the result matters to the public thesis of the project
