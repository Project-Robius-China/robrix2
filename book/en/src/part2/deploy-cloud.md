# Using Cloud Matrix: Meldry or the Official Node

> **Scope**: This chapter swaps the "communication substrate" in the architecture for a cloud service, keeping only agent-chat and Robrix2 local. Prerequisites: the route choice in Chapter 4.

Don't want to maintain a homeserver yourself? The deployment of agent-chat and Robrix2 stays the same (follow steps 2 and 3 of the [previous chapter](deploy-local.md)); only the Matrix server address now points to the cloud.

## Option A: Meldry — Create Your Own Matrix Server in One Click

[Meldry](https://tenant.meldry.com/) is a managed Matrix service built on Palpo: after signing up you can **create a Matrix server (tenant) of your own**, with its own server domain and no operational overhead whatsoever.

1. Open <https://tenant.meldry.com/> and sign up;
2. Create your tenant (Matrix server) and get its dedicated server address;
3. Prepare two kinds of accounts on that server: your human account (create it through Robrix2's registration screen), and the agent-chat bridge bot account (the username is determined by `MATRIX_BOT_USERNAME` in `.env`, default `agent-bridge`, with an optional custom suffix; on servers with open registration the bridge registers itself, otherwise pre-register it or configure a registration token);
4. Edit agent-chat's `.env`: point the Matrix server address and the bridge bot credentials at your Meldry tenant;
5. When logging in to Robrix2, enter your tenant address as the Homeserver.

This route combines "your own server" (dedicated domain, dedicated data, dedicated administration) with "zero operations".

## Option B: The Official matrix.org Node

You can also use a public homeserver such as [matrix.org](https://matrix.org) directly: register the human account and the bridge bot account, and point both agent-chat's `.env` and Robrix2's login at `https://matrix.org`.

Two caveats:

- Public nodes impose **rate limits** on registration and message sending. Bridge bot traffic is relatively chatty and may trip the limiter (agent-chat's bridge has built-in rate control, but the experience still won't match a dedicated server);
- Collaboration data in unencrypted rooms is stored on the public server; approval DMs are always end-to-end encrypted and the server cannot read their contents — this guarantee holds regardless of who owns the server.

## How to Choose

| | Local Palpo | Meldry tenant | matrix.org |
|---|---|---|---|
| Data ownership | ✅ Full | ◐ Dedicated tenant | ✗ |
| Operational cost | Requires Docker | Zero | Zero |
| Cross-device / cross-network access | Expose it yourself | ✅ Public by default | ✅ |
| Rate limits | You set the rules | Lenient | Strict |

For solo development, local Palpo is the least hassle; when you want remote teammates (humans, or someone else's agent team on another machine) to join the same space, a Meldry tenant is the most balanced choice — the two remote agent teams collaborating in one room in the Chapter 5.2 screenshots rely on exactly this: a publicly reachable Matrix server.
