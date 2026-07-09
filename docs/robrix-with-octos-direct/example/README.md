# Direct Octos — runnable example

A minimal, copy-paste-runnable Octos profile that logs into Matrix as a regular
user account (Direct mode) and connects to Robrix. Full walkthrough:
[中文](../01-connecting-your-own-octos-to-robrix-zh.md) ·
[English](../01-connecting-your-own-octos-to-robrix.md).

## Files

| File | What it is |
|---|---|
| `myagent.example.json` | The Octos gateway **profile** (`--profile`). Copy to `myagent.json` and edit. |
| `.env.example` | Holds your `DEEPSEEK_API_KEY`. Copy to `.env` and fill in. |
| `start.sh` | Loads `.env`, sets the proxy guard, runs `octos gateway`. |

## Run it in 3 steps

```bash
# 1. Create your profile and fill in the 4 marked values
cp myagent.example.json myagent.json
#    edit: homeserver, server_name, user_id, password

# 2. Create your env file and add your LLM key
cp .env.example .env
#    edit: DEEPSEEK_API_KEY

# 3. Start (foreground; Ctrl-C to stop)
./start.sh
```

Success looks like this line in the output:

```
INFO Matrix user channel authenticated user_id=@myagent:example.org
```

Then add the agent in Robrix: **Settings → Labs → Agent Access → Add an agent →
Octos (Direct) → enter your bot's Matrix ID → Add friend & bind**.

## The 5 values you must edit in `myagent.json`

| Field | Example | Yours |
|---|---|---|
| `homeserver` | `https://matrix.example.org` | your homeserver's CS-API URL (with scheme/port) |
| `server_name` | `example.org` | the part after the colon in your MXIDs |
| `user_id` | `@myagent:example.org` | your **bot** account's full Matrix ID |
| `password` | `REPLACE_WITH_...` | that account's password |
| `allowed_senders` | `["@you:example.org"]` | **your own** Matrix ID(s) — who may use the agent (see below) |

> The other fields are already set for a "personal assistant" that auto-joins
> invites and replies to everyone. These override Octos's defaults on purpose —
> by default a Direct agent does **not** auto-join (`auto_join` defaults to
> `off`), needs an allowlist in rooms (`group_policy` defaults to `allowlist`),
> and only replies when @-mentioned (`require_mention` defaults to `true`). See
> the main guide for what each field does and how to lock the agent down.
>
> Leave the rest as-is: `id`/`name`/`enabled`, the `llm` block, and
> `created_at`/`updated_at` are profile metadata. The timestamps are **required**
> for the profile to load — keep them (any valid RFC 3339 value works).

## Security — `allowed_senders` is your main gate

This example is deliberately permissive: `auto_join: always` + `group_policy: open`
+ `require_mention: false` mean the agent joins any room it's invited to and replies
to messages there. With that posture, **`allowed_senders` is the only thing deciding
who can actually drive your agent — and spend your LLM API budget.**

- `["@you:example.org"]` (the default here) — **only you** can trigger it. Add more
  Matrix IDs for teammates you trust: `["@you:example.org", "@teammate:example.org"]`.
- `[]` (empty) — **anyone** in a joined room can use it. On a federated or shared
  homeserver that means any stranger who learns the bot's ID can invite it and run
  up your bill. Only use `[]` on a private, trusted server.

Also: `myagent.json` and `.env` contain a password / API key. Do **not** commit them —
only the `*.example` templates belong in git.
