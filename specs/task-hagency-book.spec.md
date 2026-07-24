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
  issue-workflow, threads, operations, security-model) 900–2200 汉字 each,
  depth 3 layers (What → mechanism/Why → constraints & security rationale);
  remaining chapters 500–1400 汉字. Total zh body 14k–24k 汉字. Operational
  detail belongs in the operations chapter instead of being repeated everywhere.
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
- Capability claims use one of four maturity levels: protocol-enforced, current
  implementation, workflow convention, or planned. Screenshots are field
  evidence, not proof of universal behavior.
- The book distinguishes room→group, (room, agent)→owner, approval-store, and
  group→project/workflow bindings; Robrix2 is never an authorization source.
- Project Board remains explicitly labeled as a branch preview until its commit
  is merged into the documented agent-chat release.

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

<!-- lint-ack: verification-metadata-suggestion — real filesystem trees are compared directly; metadata below records the verification target -->
Scenario: Structural parity between languages
  Test: diff chapter file lists of zh and en
  Level: filesystem
  Test Double: none; compare the real bilingual source trees
  Targets: book/zh/src, book/en/src, both SUMMARY.md files
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

<!-- lint-ack: verification-metadata-suggestion — this is an explicit manual review of pinned production artifacts, with no test double -->
Scenario: Deployment facts match reality
  Test: manual review against repositories
  Level: manual
  Test Double: none; inspect the pinned real robrix2 and agent-chat repositories
  Targets: deploy chapters, agent-chat installer/CLI/env/bridge, Palpo compose
  Given the deploy chapters
  When commands, ports, and paths are checked against robrix2 and agent-chat
  Then no command, port, or path contradicts the repositories

Scenario: Security boundaries are explicit
  Test: manual review against owner, approval, ACL, and room-encryption code
  Given the deployment, approval, and security chapters
  When an evaluator follows the documented trust and owner sequence
  Then a human invite establishes the exact owner MXID
  And missing or ambiguous owner state denies without an admin fallback
  And project-room and approval-room encryption boundaries are not conflated

Scenario: Workflow maturity is not overstated
  Test: grep maturity labels and review against issue-workflow and Project Board
  Given the workflow, thread, pool, and Project Board chapters
  When a behavior depends on a skill, runtime profile, or preview branch
  Then it is not described as a backend-enforced shipped workflow

<!-- lint-ack: verification-metadata-suggestion — the runbook is checked against the real service boundaries and stores named below -->
Scenario: Failure paths are actionable
  Test: review the operations decision trees
  Level: manual
  Test Double: none; trace the real bridge/backend/runtime stores and logs
  Targets: operations chapter, bridge, backend, push relay, approval audit
  Given a missing reply, missing approval card, misplaced thread reply, or expiry
  When the reader opens the operations chapter
  Then the checks identify the authoritative log/store at every pipeline layer

Scenario: Claims end with verifiable boundaries
  Test: scan chapter conclusions and guarantee language
  Given the bilingual chapter conclusions
  When tone and guarantee claims are reviewed
  Then no chapter ends with a motivational slogan
  And every security or product guarantee states its implementation preconditions
