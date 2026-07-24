# Core IM Features at a Glance

> **Scope**: This chapter is a quick tour of Robrix2's feature set as an IM. Prerequisite: Chapter 1. These features are also the foundation for every collaboration capability in Part II — each one will be put to work later in the book.

## Conversations and Messages

- **Rooms / DMs / Invites**: Rooms, People, and Invites are listed together in the left sidebar; incoming invites can be joined or rejected in place and take effect immediately, no restart required.
- **Threads**: any message can be expanded into its own thread; a thread opens in its own tab, and the Threads panel on the right gives an overview of every thread in a room. All of Chapter 5.3 is built on this feature.
- **Rich text and Markdown**: the input box supports Markdown natively; messages render as HTML (`formatted_body`) — code blocks, lists, bold, the works. Agents' structured reports depend on it entirely.
- **@mentions**: typing `@` pops up a room-member picker that completes humans and bot accounts alike; messages that mention you are highlighted.
- **Slash commands**: typing `/` opens the command palette — room management commands, bot commands, and the workflow commands introduced in Chapter 5.2.
- **Reply / edit / forward / delete**: the full message operation set, with edit and reply relationships strictly following the Matrix specification.
- **Attachments and media**: send and receive images and files, with image previews and link-card previews.

## Security and Accounts

- **End-to-end encryption (E2EE)**: encrypted rooms work out of the box, backed by matrix-rust-sdk's Megolm/Olm implementation; device management lives in Settings → Devices. The approval DMs in Chapter 5.4 depend entirely on this layer.
- **Multilingual UI**: built-in i18n (English / Simplified Chinese).
- **Live translation (Labs)**: configure an OpenAI-compatible API to translate messages in real time right in the input bar — a direct win for cross-language teams (or cross-language human–agent collaboration).

## Workspace

- **Multi-tab dock**: rooms, threads, and approval DMs each get their own tab, spreading multiple collaboration sites across one screen;
- **Global search**, **unread badges**, **desktop notifications**.

That concludes Part I. Starting with the next chapter, we turn this IM into HAgency's human workbench.
