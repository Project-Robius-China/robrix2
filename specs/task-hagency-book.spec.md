spec: task
name: "HAgency Book (Robrix2 x agent-chat, bilingual mdBook)"
inherits: project
tags: [book, docs, hagency, agent-chat, i18n]
estimate: 3d
---

## Intent

Produce a bilingual (zh-CN primary, en mirror) mdBook under `book/` that
(1) briefly introduces Robrix2 as a Matrix IM client, and (2) deeply documents
HAgency — the Robrix2 + agent-chat + Palpo collaboration system where humans
remain the agentic subject: deployment (local and cloud Matrix), and
screenshot-driven team-collaboration workflows. All claims about system
behavior must match the real implementations in the robrix2 and agent-chat
repositories; screenshots are real captures, not mockups.

## Decisions

- Book lives in `book/` with two independent mdBooks: `book/zh/` (source of
  truth) and `book/en/` (structural 1:1 mirror, content-equivalent translation,
  not word-for-word).
- Writing budget (tech-writer): Part I chapters 500–900 汉字 each, depth 2
  layers; Part II core chapters (concept, deploy-local, approvals,
  issue-workflow, threads, security-model) 900–1800 汉字 each, depth 3 layers
  (What → mechanism/Why → constraints & security rationale); remaining chapters
  500–1000 汉字. Total zh body 12k–18k 汉字. A chapter exceeding budget by
  >30% must cut depth, not add chapters.
- Every chapter opens with a positioning anchor blockquote (`> **定位**` /
  `> **Scope**`): one-sentence topic, prerequisite chapter, applicable reader.
- Mermaid via `mdbook-mermaid`: at least one diagram in concept (architecture),
  deploy-local (component/port topology), threads (message flow),
  approvals (approval sequence), issue-workflow (role state flow).
- Screenshots: the 14 real captures under `src/images/` (11 Robrix2 client
  captures + 3 agent-chat Project Board dashboard captures) are shared by copy
  into both language trees; every screenshot is referenced by at least one
  chapter and captioned with what it evidences.
- Version baseline note lives once in the preface (system under active
  development), not per-chapter.
- Deployment instructions must match the real artifacts: robrix2
  `palpo-and-octos-deploy/` compose (Palpo :8128), agent-chat CLI
  (`agentchat service/up/down/ls`, backend :8090, dashboard :8084), Meldry
  (https://tenant.meldry.com/) and matrix.org as cloud options.
- Security claims must reflect implemented behavior only: owner-approval
  protocol (`com.agentchat.approval.*`), fail-closed semantics, one-shot
  consume, E2EE approval rooms, sender/room server-side validation, managed
  runtime launch. No aspirational features presented as shipped.
- Tone: no motivational endings, no preemptive apologies; ratings and
  guarantees stated with preconditions.

## Boundaries

### Allowed Changes
- book/
- specs/task-hagency-book.spec.md
- .github/workflows/deploy-book.yml

### Forbidden
- Do not modify robrix2 or agent-chat source code
- Do not invent features, commands, ports, or protocol fields not present in
  the repositories
- Do not use placeholder/mock screenshots
- Do not commit or publish without user review

## Out of Scope

- API reference documentation for agent-chat
- Robrix2 end-user manual for pure IM usage beyond one overview chapter

## Completion Criteria

Scenario: Both language trees build
  Test: mdbook build book/zh && mdbook build book/en
  Given the bilingual book structure
  When mdbook build runs in each language directory
  Then both builds succeed with no broken internal links

Scenario: Structural parity between languages
  Test: diff chapter file lists of zh and en
  Given zh/src and en/src
  When their SUMMARY.md chapter lists are compared
  Then every zh chapter has exactly one en counterpart with the same path

Scenario: Positioning anchors present
  Test: grep positioning anchor in every chapter
  Given all chapter files in both trees
  When scanned for the anchor blockquote
  Then every chapter (excluding SUMMARY) contains one

Scenario: Mermaid coverage
  Test: grep mermaid fences in the five designated chapters
  Given concept, deploy-local, threads, approvals, issue-workflow
  When scanned for ```mermaid blocks
  Then each contains at least one diagram and mdbook-mermaid renders it

Scenario: Screenshot coverage
  Test: cross-check images referenced from chapters
  Given the 14 screenshots in src/images
  When chapter image references are collected
  Then all 14 are referenced at least once in each language tree

Scenario: Deployment facts match reality
  Test: manual review against repositories
  Given the deploy chapters
  When commands, ports, and paths are checked against robrix2 and agent-chat
  Then no command, port, or path contradicts the repositories
