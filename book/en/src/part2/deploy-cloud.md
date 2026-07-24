# Using Cloud Matrix: Meldry or the Official Node

> **Scope**: This chapter swaps the "communication substrate" in the architecture for a cloud service, keeping only agent-chat and Robrix2 local. Prerequisites: the route choice in Chapter 4.

Don't want to maintain a homeserver yourself? The deployment of agent-chat and Robrix2 stays the same (follow steps 2 and 3 of the [previous chapter](deploy-local.md)); only the Matrix server address now points to the cloud.

## Option A: Managed Matrix with Meldry

[Meldry](https://tenant.meldry.com/) offers managed Matrix tenants based on Palpo. Capabilities, data boundaries, rate limits, and pricing must be checked against its current service documentation; this book only assumes a working Client-Server API and a way to provision the required accounts.

1. Open <https://tenant.meldry.com/> and sign up;
2. Create your tenant (Matrix server) and get its dedicated server address;
3. Prepare three account classes: human users, the bridge bot, and one `@ac_*` puppet per Agent. The server must allow agent-chat to provision the latter two through a supported registration flow/token, or an administrator must pre-create them;
4. Edit agent-chat's `.env`: point the Matrix server address and the bridge bot credentials at your Meldry tenant;
5. When logging in to Robrix2, enter your tenant address as the Homeserver.

Then follow the same local sequence: configure secrets, start agent-chat, have the human owner invite every actual Agent, and bind the project room. A managed homeserver does not establish owner provenance for you.

## Option B: The Official matrix.org Node

You can evaluate a public homeserver such as [matrix.org](https://matrix.org), but the ability to register one human account does not imply permission to automate a bridge plus many puppet accounts. Without a compatible registration flow or pre-provisioning, this deployment cannot work as written.

Two caveats:

- Public nodes impose **rate limits** on registration and message sending. Bridge bot traffic is relatively chatty and may trip the limiter (agent-chat's bridge has built-in rate control, but the experience still won't match a dedicated server);
- Current project rooms must be unencrypted, so their content is stored by the homeserver and may federate. Approval bodies are E2EE, while membership, timing, and traffic metadata remain visible.

## How to Choose

| | Local Palpo | Meldry tenant | matrix.org |
|---|---|---|---|
| Data control | Self-hosted | Depends on service terms | Public service terms |
| Homeserver operations | You operate it | Provider operates it | Provider operates it |
| Cross-device / cross-network access | Expose it yourself | ✅ Public by default | ✅ |
| Registration and limits | You configure them | Verify tenant policy | Verify public-node policy |

Choose by four verifiable conditions: account provisioning, stable Client-Server APIs, bridge-compatible limits, and an acceptable storage location for unencrypted project data. Cross-homeserver collaboration additionally requires correct public DNS, TLS, and Matrix federation.

For internet-facing dashboards, use an HTTPS reverse proxy, set `AGENT_CHAT_WEB_URL`, and keep the backend API loopback-only unless you deliberately add access control.
