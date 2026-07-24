# 把 Agent 请进你的空间

> **定位**：本章完成「空间里出现第一个 Agent」：Robrix2 侧的 Agent 识别设置，与 agent-chat 侧的房间邀请。前置依赖：第 4 章。

## Agent Access：Robrix2 的智能体接入面板

打开 **Settings → Labs → Agent Access**。这里是 Robrix2 管理智能体的入口：绑定一个 Matrix 账号、标记它属于哪个 Agent 框架，之后 Robrix2 就能在所有房间里识别它 —— 加机器人徽标、启用对应的斜杠命令。

![Agent Access 设置页](../images/agent-access-settings.png)

面板分三块：

- **AppService 绑定**：Robrix2 保持普通 Matrix 客户端的身份，但可以绑定一个 AppService（截图中为 Octos AppService），并运行与之匹配的斜杠命令；
- **Registered agents**：已注册 Agent 列表，每个条目可 Open chat / Re-check / Unbind；
- 下方还有 **Real-time Translation** 等 Labs 功能。

## 添加一个 Agent：选择框架

点 **Add an agent**，第一步选择该账号背后的 Agent 框架：

![Add an agent 框架选择](../images/add-agent-modal.png)

- **Octos（AppService）**：注册在服务器上的应用服务；
- **Octos（Direct）／Hermes／OpenClaw**：以「Matrix 好友」形式直接添加的 Direct Agent。

区分这两类的意义在于能力边界：AppService 由服务器托管、可以管理自己名下的一批账号；Direct Agent 就是一个普通 Matrix 账号背后的机器人。Robrix2 对两类都只做**识别与展示**，不参与它们的执行。

> agent-chat 的 Agent 不需要在这里手工添加 —— 它们的木偶账号（`@ac_…`）由桥自动注册并拉进房间，Robrix2 会按名字模式自动识别。

## 接受桥的邀请

agent-chat 桥会以桥机器人身份（`@agent-bridge-<你的名字>`）把你邀请进它管理的房间：项目房间、审批私聊等。邀请出现在 Robrix2 左侧的 **Invites** 区，点 **Join Room** 即可：

![来自桥机器人的房间邀请](../images/bridge-invite.png)

> 截图左栏能看到多个不同的桥（`agent-bridge-alexlocal`、`agent-bridge-alan`、`agent-bridge-tyrese`）发来的邀请 —— 每个人类用户跑自己的 agent-chat 实例、管自己的 Agent 团队，但都汇入同一个 Matrix 空间。这正是开放协议下多实例协作的样子，下一章会看到它们同房工作的场面。
