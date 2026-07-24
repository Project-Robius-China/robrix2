# Inviting Agents into Your Space

> **Scope**: This chapter separates Robrix2's generic Agent Access registry from agent-chat puppet and owner onboarding. Prerequisite: Chapter 4.

## Agent Access: Robrix2's Agent Integration Panel

Open **Settings → Labs → Agent Access**. This is Robrix2's generic Agent Registry: bind a Matrix account and tag its framework for badges, status, and framework integrations. It is not agent-chat's owner database and grants no approval authority.

![Agent Access settings page](../images/agent-access-settings.png)

The panel has three sections:

- **AppService binding**: Robrix2 remains a plain Matrix client, but it can bind to an AppService (the Octos AppService in the screenshot) and run the slash commands that go with it;
- **Registered agents**: the list of registered Agents, each with Open chat / Re-check / Unbind actions;
- Below that are other Labs features such as **Real-time Translation**.

## Adding an Agent: Choosing a Framework

Click **Add an agent**. The first step is to choose which Agent framework sits behind the account:

![Add an agent framework picker](../images/add-agent-modal.png)

- **Octos (AppService)**: an application service registered on the server;
- **Octos (Direct) / Hermes / OpenClaw**: Direct Agents added like ordinary "Matrix friends".

The distinction matters because of capability boundaries: an AppService is hosted by the server and can manage a whole fleet of accounts under its name; a Direct Agent is just a bot behind a regular Matrix account. For both kinds, Robrix2 only does **recognition and display** — it plays no part in their execution.

agent-chat currently follows a separate path:

- it registers known `@ac_<name>` puppets but does not automatically add them to arbitrary project rooms;
- Robrix2 does not add an account to its generic Agent Registry merely because the name starts with `@ac_`;
- name patterns are mainly used to discover `*_coordinator` for workflow text completion, not authentication;
- owner provenance comes from the full `event.sender` MXID that invites the actual puppet.

The Octos/Hermes/OpenClaw screenshot demonstrates generic Agent Access, not a completed agent-chat binding.

## Correct Invitation Order

In the unencrypted project room:

1. a trusted inviter adds the companion bridge;
2. an operator sends `!bindroom <existing-group>`;
3. **you personally invite each `@ac_<agent>` puppet**;
4. wait for invite polling and verify both Agent and companion bridge are joined;
5. accept the `Approval: <agent>` invitation.

Step 3 establishes `(room, agent) → owner`. A bridge-created room or bridge-issued project invite cannot establish “the human who invited the Agent owns it.”

## Accepting Approval-Room Invitations

The bridge creates an approval room on demand for `(agent, owner)` and invites you. Click **Join Room** under Invites:

![Room invitation from the bridge bot](../images/bridge-invite.png)

> Bridge names alone do not prove owner provenance. Verify who invited which actual Agent and which `(agent, owner)` the approval room represents. Ordinary DMs are for assignments; approval rooms accept only structured verdicts.
