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
| `.gitignore` | Prevents the generated credentials and runtime data from being committed. |

## Install a compatible Octos binary

The Matrix user-account channel landed in
[Octos PR #1475](https://github.com/octos-org/octos/pull/1475), after the
`v1.1.0` tag. Until a newer release explicitly includes that PR, build a
current checkout with the Matrix feature enabled:

```bash
OCTOS_SRC="$(mktemp -d)/octos"
git clone https://github.com/octos-org/octos.git "$OCTOS_SRC"
cd "$OCTOS_SRC"
git merge-base --is-ancestor 355147f1 HEAD || {
  echo "This checkout does not contain Octos PR #1475" >&2
  exit 1
}
cargo install --path crates/octos-cli --locked --features "api,matrix" --force
octos --version
```

Then return to this example directory. A plain `v1.1.0` release binary cannot
run this profile.

## Run it in 3 steps

```bash
# 1. Create your profile and fill in its 4 account/access values
cp myagent.example.json myagent.json
#    edit: homeserver, user_id, password, allowed_senders

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

Use an unencrypted DM or room. The current Octos Matrix user-account channel
does not decrypt `m.room.encrypted` events.

Then add the agent in Robrix: **Settings → Labs → Agent Access → Add an agent →
Octos (Direct) → enter your bot's Matrix ID → Add friend & bind**.

## The 5 required values

Four values are in `myagent.json`:

| Field | Example | Yours |
|---|---|---|
| `homeserver` | `https://matrix.example.org` | your homeserver's CS-API URL (with scheme/port) |
| `user_id` | `@myagent:example.org` | your **bot** account's full Matrix ID |
| `password` | `REPLACE_WITH_...` | that account's password |
| `allowed_senders` | `["@you:example.org"]` | **your own** Matrix ID(s) — who may use the agent (see below) |

The fifth value is `DEEPSEEK_API_KEY` in `.env`. If a global proxy must bypass
a local/LAN homeserver, also set the optional `MATRIX_NO_PROXY_HOST` to the
hostname or IP from `homeserver`.

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

Also: `myagent.json` and `.env` contain a password / API key. The bundled `.gitignore`
protects the standard filenames, but still verify `git status` before committing. Only
the `*.example` templates belong in git.
