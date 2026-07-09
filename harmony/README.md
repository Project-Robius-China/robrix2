# Robrix 鸿蒙 (HarmonyOS / OpenHarmony) 打包与运行完全指南

本文档记录把 **Robrix** 交叉编译、打包、签名并运行到 **HarmonyOS 模拟器**（以及 OpenHarmony 真机）上的全过程，包含踩过的每一个坑、对应修复、签名细节、已知限制和故障排查。

工具链：`cargo-makepad`（makepad 官方的 `cargo makepad ohos` 子命令）+ DevEco Studio 自带的 OpenHarmony SDK。

---

## 0. 现状 (2026-07-09)

| 项 | 状态 |
|----|------|
| 交叉编译 `aarch64-unknown-linux-ohos` | ✅ 通过 |
| hvigor 打包 HAP | ✅ 通过 |
| 签名（HAP 可被模拟器接受安装） | ✅ 通过（用 SDK 自带 OpenHarmony 调试材料，无需华为账号） |
| 安装 + 启动到模拟器 | ✅ 通过（`Huawei_Phone`，HarmonyOS 5.1.0 / API 18，arm64） |
| **图标 / 头像 / 纹理渲染** | ✅ 通过（**必须** `MAKEPAD=ohos_sim`） |
| **文字 (glyph) 渲染** | ⚠️ **模拟器上不显示**（makepad SLUG 走 RGBA32F 浮点纹理，模拟器虚拟 GLES 不支持；真机大概率正常）见 §6.1 |

一键运行：`harmony/ohos.sh run`。

---

## 1. 环境准备

- **DevEco Studio**（macOS）：`/Applications/DevEco-Studio.app`。里面自带了全部需要的东西：
  - OpenHarmony SDK：`Contents/sdk/default/openharmony`
  - 交叉编译 clang：`.../native/llvm/bin/aarch64-unknown-linux-ohos-clang`
  - sysroot：`.../native/sysroot`
  - `hdc`：`.../toolchains/hdc`
  - 签名工具与材料：`.../toolchains/lib/{hap-sign-tool.jar, OpenHarmony.p12, OpenHarmonyProfileDebug.pem, UnsgnedDebugProfileTemplate.json}`
  - node / hvigor：`Contents/tools/{node,hvigor}`
  - 模拟器：`Contents/tools/emulator/Emulator`
  - JBR（java/keytool）：`Contents/jbr/Contents/Home/bin`
- **Rust**：nightly + `aarch64-unknown-linux-ohos` target（`cargo makepad ohos install-toolchain` 会装）。注意 `cargo-makepad` 内部固定用 `rustup run nightly`，所以仓库的 `rust-toolchain.toml`（pin 到 stable 1.94）不影响 OHOS 构建。
- **cmake**（`brew install cmake`）：`aws-lc-sys` 交叉编译需要。
- **模拟器**：DevEco → Device Manager 里已部署的 `Huawei_Phone`（HarmonyOS 5.1.0 `phone_all_arm`，arm64）。

关键环境变量（`harmony/ohos.sh` 会自动设好）：
```bash
DEVECO_HOME=/Applications/DevEco-Studio.app/Contents
# aws-lc-sys 的 bindgen 需要 OHOS sysroot（见 §4.1）
BINDGEN_EXTRA_CLANG_ARGS_aarch64_unknown_linux_ohos="--target=aarch64-linux-ohos --sysroot=$DEVECO_HOME/sdk/default/openharmony/native/sysroot"
# 模拟器渲染必须（见 §4.4）
MAKEPAD=ohos_sim
```

---

## 2. 快速开始

```bash
# 1) 启动模拟器（首次要接受协议）
EMU=/Applications/DevEco-Studio.app/Contents/tools/emulator/Emulator
"$EMU" -license accept
"$EMU" -start Huawei_Phone        # 也可以在 DevEco 的 Device Manager 里点 Start
# 确认已连上：
/Applications/DevEco-Studio.app/Contents/sdk/default/openharmony/toolchains/hdc list targets

# 2) 交叉编译 + 生成 DevEco 工程（首次，或改了 Rust 代码后）
harmony/ohos.sh deveco

# 3) 打包 + 签名 + 安装 + 启动（日常一条命令）
harmony/ohos.sh run

# 其它
harmony/ohos.sh logs      # 看 hilog
harmony/ohos.sh shot      # 截图到 harmony/robrix_screen.jpeg
```

> 真机构建（关掉模拟器专用的 ohos_sim）：`MAKEPAD= harmony/ohos.sh run`

### 2.1 真机 vs 模拟器（重要，别搞混）

**核心一句话：真机构建务必 `MAKEPAD=` 留空，别带 `ohos_sim`。**

| | 模拟器 | 真机 |
|---|--------|------|
| **构建命令** | `harmony/ohos.sh run`（默认已带 `MAKEPAD=ohos_sim`） | `MAKEPAD= harmony/ohos.sh run`（**MAKEPAD 留空**） |
| **为什么** | 模拟器虚拟 GLES 需要 `ohos_sim`（全量纹理上传 + 模拟器 EGL），否则图标/文字全黑（§4.4 / §6.1） | `ohos_sim` 是模拟器专用；真机带上它会走错渲染路径，可能**渲染异常 / 变慢**，且真机 GPU 支持浮点纹理，本来就不需要它 |
| **签名** | SDK 自带 OpenHarmony 调试证书即可，脚本全自动（§5） | 鸿蒙 NEXT 真机多半**不认** OpenHarmony 调试证书，需 **DevEco 自动签名（华为账号）+ 开发者模式 + 设备注册**（见下） |
| **文字渲染** | ⚠️ 不显示（浮点纹理限制，§6.1） | ✅ 预期正常（真机 GPU 支持 RGBA32F） |

- **脚本自带防呆**：`ohos.sh` 在构建前会用 `hdc list targets` 检测连的是模拟器（`127.0.0.1:*`）还是真机（USB 串号）。如果和 `MAKEPAD` 不匹配（比如插着真机却带 `ohos_sim`，或空 `MAKEPAD` 却连着模拟器），会打印醒目 `WARNING` 提示你换正确命令（只告警、不拦截）。
- **真机签名怎么弄**：把生成的工程 `target/makepad-open-harmony/robrix/` 用 DevEco Studio 打开 → `File` → `Project Structure` → `Signing Configs` → `Sign in`（华为账号）→ `Apply`。DevEco 会自动往 `build-profile.json5` 写 `signingConfigs`，之后 hvigor 自己签名 —— 这时**不要**再用 `ohos.sh sign`，直接让 DevEco 部署或 `harmony/ohos.sh build` + `hdc install`。⚠️ 别再跑 `harmony/ohos.sh deveco`（会重建工程、清掉刚写好的签名配置）。

---

## 3. 完整流程详解

`cargo makepad ohos` 的四个阶段（`harmony/ohos.sh` 封装了它们）：

1. **`deveco`** = `rust_build`（把 robrix 编成 `librobrix.so` cdylib）+ `create_deveco_project`（在 `target/makepad-open-harmony/robrix/` 生成一个 DevEco 工程模板，模板来自 cargo-makepad 内置的 `tools/open_harmony/deveco`）。
2. **`build`** = 再跑 `rust_build`（有缓存）+ `add_dependencies`（把 `librobrix.so` 复制成 `entry/libs/arm64-v8a/libmakepad.so`，并把各 crate 的 `resources/`（字体等）拷进 `entry/.../rawfile/`）+ `build_hap`（node 跑 hvigor `assembleHap`，产出 `.../outputs/default/makepad-default-unsigned.hap`）。
3. **`sign`**（我们自己加的，见 §5）：用 SDK 自带 OpenHarmony 调试材料签名，产出 `makepad-default-signed.hap`。
4. **`deploy`** = `hdc` 推包 + `bm install` + `aa start -a EntryAbility -b dev.makepad.robrix`。

产物路径：
```
target/aarch64-unknown-linux-ohos/debug/librobrix.so                 # cdylib（debug ~1.2GB）
target/makepad-open-harmony/robrix/                                   # DevEco 工程
  entry/libs/arm64-v8a/libmakepad.so                                 # = librobrix.so
  entry/build/default/outputs/default/makepad-default-unsigned.hap   # 未签名 (~210MB)
  entry/build/default/outputs/default/makepad-default-signed.hap     # 已签名
bundle id = dev.makepad.robrix，ability = EntryAbility
```

---

## 4. 踩过的坑与修复

根因几乎都是同一个：**OHOS 复用了 `target_os = "linux"`**，导致一堆“桌面 Linux 专属”的依赖/代码被错误地编进 OHOS 构建。

### 4.1 `aws-lc-sys` 交叉编译失败（bindgen 找不到 `<stdlib.h>`）

- 来源：matrix-sdk 的 `rustls-tls` → `aws-lc-rs` → `aws-lc-sys`。
- 报错：`fatal error: 'stdlib.h' file not found`。bindgen 直接调 libclang，不带 OHOS sysroot。
- 修复：给 bindgen 指定 sysroot（**无需改 Cargo**）：
  ```bash
  export BINDGEN_EXTRA_CLANG_ARGS_aarch64_unknown_linux_ohos="--target=aarch64-linux-ohos --sysroot=<DEVECO>/sdk/default/openharmony/native/sysroot"
  ```
- 已固化在 `harmony/ohos.sh`。

### 4.2 `rfd`（文件对话框）拉入 wayland / ashpd / zbus，无法交叉编译

- 来源：`Cargo.toml` 里 `rfd = "0.15"`，原本 gate 在 `cfg(any(macos, windows, linux))` —— **OHOS 命中 linux**，于是被编进来。`rfd` 在 linux 无 headless 后端，必然拉 wayland/ashpd/zbus。
- 报错：`wayland-sys ... pkg-config has not been configured to support cross-compilation`。
- 修复（**已改进仓库**）：
  - `Cargo.toml`：把 `rfd` 单独 gate 到 `cfg(all(any(macos,windows,linux), not(target_env = "ohos")))`。
  - 5 处调用点（`src/settings/account_settings.rs`、`src/home/create_room.rs`、`src/home/room_settings_modal.rs`、`src/room/room_input_bar.rs`、`src/shared/attachment_download.rs`）把 cfg 收窄，让 **OHOS 走和 Android/iOS 一样的 mobile stub**（文件选择在移动端本来就是“提示暂不支持”）。

### 4.3 makepad 本身的 4 个 OHOS 编译 bug

Robrix pin 的 makepad `dev` commit（`3d18a137`）在 OHOS 上编不过。都是 “把 OHOS 当桌面 Linux” 的漏网。补丁：

| 文件 | 问题 | 修复 |
|------|------|------|
| `platform/src/cx_api.rs` (~L1702) | `can_play_type_impl` 在 `cfg(all(linux, not(android)))`（含 ohos）里引用了不存在的 `linux_video_playback` 模块 → `E0433` | cfg 加 `not(target_env="ohos")`，并给 ohos 加一个返回 `""` 的桩 |
| `platform/src/gl_render_bridge.rs` (~L124) | `#[cfg(target_os="linux")] impl Cx`（含 ohos）访问了 `os.opengl_cx`，而 OHOS 的 `CxOs` 没这个字段 → `E0609` | cfg 改成 `all(linux, not(target_env="ohos"))`（这套 GL bridge 只给桌面外部渲染器用，robrix 用不到） |
| `platform/build.rs` (~L172) | `"linux" =>` 分支无条件 `cargo:rustc-link-lib=xkbcommon`（含 ohos），OHOS sysroot 无 libxkbcommon → 链接失败 | `if !target.ends_with("-ohos")` 时才 link |
| `platform/network/src/{backend.rs, socket_stream.rs}` | Linux 网络后端 `#[link(name="ssl"/"crypto")]`（含 ohos），OHOS 无 OpenSSL → 链接失败 | 把 ohos 排除出 linux 后端，路由到 `UnsupportedBackend` / 复用 wasm 的桩（robrix 走 reqwest，不用 makepad-network） |

> ✅ 这 5 处补丁已固化到 makepad fork：**`Project-Robius-China/makepad` 分支 `robrix-ohos-3d18a137`**（= robrix 现在 pin 的 `3d18a137` + 上述 OHOS 修改），robrix 的 `Cargo.toml` 通过 `[patch]` 指向它的固定 commit。**别人拉了 robrix 分支就会自动带上补丁，无需手动改 cargo 缓存。** 详见 §6.2。

### 4.4 图标 / 头像 / 纹理全变黑：`MAKEPAD=ohos_sim`

- 现象：编译链接都过、App 能起、布局和纯色（teal 按钮）都对，但**图标、头像、图片纹理全渲染成黑块**。
- 根因：模拟器的**虚拟 GLES 不支持 makepad 的“部分纹理更新”**。makepad 源码原话：
  ```rust
  // platform/src/os/linux/opengl.rs:2564
  // "OHOS simulators/emulators still need the conservative full upload path."
  const DO_PARTIAL_TEXTURE_UPDATES: bool = cfg!(not(ohos_sim));
  ```
  makepad 的 OHOS README 也明说：给模拟器/仿真器构建**必须** `MAKEPAD=ohos_sim`。
- 修复：构建时设 `MAKEPAD=ohos_sim`（会经 cargo-makepad 透传给 makepad 的 build.rs，打开 `ohos_sim` cfg：全量纹理上传 + 模拟器 EGL + 关闭 shader 二进制缓存）。已设为 `harmony/ohos.sh` 默认。
- ⚠️ 改了这个 cfg 需要重编 makepad-platform：`cargo clean -p makepad-platform --target aarch64-unknown-linux-ohos` 后再构建（原因见 §6.2 的指纹说明）。

---

## 5. 签名详解

HarmonyOS 模拟器**要求 HAP 已签名**才能安装。我们用 **SDK 自带的 OpenHarmony 调试材料**做**全自动 CLI 签名**（不需要华为账号）：模拟器信任这条 OpenHarmony 调试证书链。

材料（都在 `<DEVECO>/sdk/default/openharmony/toolchains/lib/`）：
- `OpenHarmony.p12`：keystore，口令 `123456`，含全套私钥（alias：`openharmony application release`、`openharmony application profile debug`、`... ca`、`... root ca`）。
- `OpenHarmonyProfileDebug.pem`：给调试 profile 签名的证书链。
- `UnsgnedDebugProfileTemplate.json`：调试 profile 模板（内含 CA 签发的 app leaf 证书）。
- `hap-sign-tool.jar`：签名工具。

步骤（`harmony/ohos.sh sign` 自动完成）：
1. **取 App 证书链**：leaf 用**模板里内嵌的那张**（issuer = `OpenHarmony Application CA`；注意**不能**用 `keytool -exportcert` 导出的那张，那张是自签的，链验证会失败），再拼上从 p12 导出的 `... ca` + `... root ca` → `app-signing-cert.pem`。
2. **生成调试 profile**：填模板 → `bundle-name = dev.makepad.robrix`、`device-ids = [模拟器 UDID]`（`hdc shell bm get --udid`）、`validity` 刷新成未来一段时间（模板自带的有效期已过期）。
3. **签 profile**：`hap-sign-tool sign-profile`（key = `openharmony application profile debug`，cert = `OpenHarmonyProfileDebug.pem`）→ `robrix.p7b`。
4. **签 HAP**：`hap-sign-tool sign-app`（key = `openharmony application release`，`-appCertFile app-signing-cert.pem`，`-profileFile robrix.p7b`，`-compatibleVersion 12 -signCode 1`）→ `makepad-default-signed.hap`。

> 每次重新打包（unsigned HAP 变了）都要重签，`harmony/ohos.sh run` 会自动做。
> **另一条路**：DevEco Studio → Project Structure → Signing Configs → Sign in（需华为账号），会自动往生成工程的 `build-profile.json5` 里写 `signingConfigs`，之后 hvigor 自己签。但注意重跑 `deveco` 会重建工程、清掉这段配置。

---

## 6. 已知问题与限制

### 6.1 ⚠️ 文字在模拟器上不显示（最主要的遗留问题）

- 现象：图标 OK 了，但**所有文字（按钮文案、房间名、输入提示…）都不渲染**。
- 根因：makepad 用 **SLUG** 渲染文字，字形图集是 **`RGBA32F` 浮点纹理**（`draw/src/text/slug_atlas.rs`，`opengl.rs:2491` 用 `GL_RGBA32F`）。模拟器的虚拟 GLES **采样不了浮点纹理** → 字形数据读成 0 → 无文字。图标能显示是因为它们走 **SDF/程序化着色器**（非浮点图集），所以 `ohos_sim` 能修图标、修不了文字。
- 判断：**很可能是模拟器 GPU 的限制**。真机（Mali/Adreno）支持 `RGBA32F`，文字大概率正常。
- 出路：
  - **A（推荐先做）**：真机验证。若真机正常，则无需改代码。
  - **B**：改 makepad 让 OHOS 走**非浮点**的 MSDF/灰度字形图集（makepad 里有 `Bgra`/RGBA8 版的 `MsdfAtlas`/`GrayscaleAtlas`），绕开浮点纹理。改动较大、需验证渲染质量。

### 6.2 makepad 补丁固化（✅ 已完成，通过 fork + `[patch]`）

§4.3 的 makepad 修改**已经**通过 fork + `[patch]` 固化到仓库，robrix 分支自洽、别人拉了直接能编：

- **fork 分支**：`Project-Robius-China/makepad` 的 `robrix-ohos-3d18a137`（= robrix pin 的 `3d18a137` + 5 处 OHOS 补丁，commit `96d2f42f9`）。
- **robrix `Cargo.toml`**：
  ```toml
  [patch."https://github.com/makepad/makepad"]
  makepad-widgets     = { git = "https://github.com/Project-Robius-China/makepad", rev = "96d2f42f91fd2d325f64d9f1a519055b231d42fa", version = "=2.0.0" }
  makepad-code-editor = { git = "https://github.com/Project-Robius-China/makepad", rev = "96d2f42f91fd2d325f64d9f1a519055b231d42fa", version = "=2.0.0" }
  ```
  （`version = "=2.0.0"` 必须加：makepad 仓库里 `makepad-code-editor` 有 1.0.0 和 2.0.0 两个候选，要消歧义。）
- 其余 makepad crate（platform/draw/…）会自动跟着 fork workspace 走，无需单独 patch。
- 因为 fork = robrix 当前 pin 的 `3d18a137` + 仅 OHOS 的 cfg 收窄，**桌面/移动端构建完全不变**。

**以后 robrix 升级 makepad**：把这 5 个补丁 rebase 到新的 pin commit，重推 fork 分支，更新上面的 `rev`；或等这些修复合进上游 makepad 后直接删掉本 `[patch]`。

> cargo 坑（供参考）：cargo 对 git 依赖**按 commit 指纹缓存，不是按文件 mtime**。直接改 `~/.cargo/git/checkouts/…` 里的文件**不会触发重编**（静默用旧缓存），必须 `cargo clean -p <crate>` 才生效 —— 这也是为什么要用 fork+`[patch]` 而不是改缓存。

### 6.3 其它

- **debug 包很大**：`librobrix.so` debug ~1.2GB，打进 HAP ~210MB。要更小/更快用 `--release`（首次编译较久）。
- **模拟器网络极慢**：ping matrix.org 往返约 12 秒，登录/同步会很卡 —— 是模拟器本身的问题，不影响打包。

---

## 7. `harmony/ohos.sh` 命令参考

```
harmony/ohos.sh deveco    # 交叉编译 librobrix.so + 生成 DevEco 工程
harmony/ohos.sh build     # hvigor 打未签名 HAP
harmony/ohos.sh sign      # 用 OpenHarmony 调试材料签名（自动取模拟器 UDID）
harmony/ohos.sh deploy    # 安装已签名 HAP 并启动
harmony/ohos.sh run       # = build + sign + deploy（常用）
harmony/ohos.sh logs      # 过滤 hilog
harmony/ohos.sh shot      # 截图到 harmony/robrix_screen.jpeg
```
可覆盖的变量：`DEVECO_HOME`、`OHOS_ARCH`（默认 aarch64）、`MAKEPAD`（默认 ohos_sim，真机用 `MAKEPAD=`）。

---

## 8. 仓库内改动清单

- **`Cargo.toml`**：① `rfd` 排除 ohos（§4.2）；② 新增 `[patch."https://github.com/makepad/makepad"]` 指向 fork（§6.2）。
- **`Cargo.lock`**：makepad 系列 crate 重定向到 fork commit `96d2f42f9`。
- **`src/…`（5 个文件）**：`src/settings/account_settings.rs`、`src/home/create_room.rs`、`src/home/room_settings_modal.rs`、`src/room/room_input_bar.rs`、`src/shared/attachment_download.rs` —— rfd 调用点 cfg 收窄（ohos → mobile 桩）。
- **新增 `harmony/`**：`ohos.sh`、`README.md`（本文）、`.gitignore`。
- **makepad 侧的 5 处 OHOS 补丁**：在 `Project-Robius-China/makepad@robrix-ohos-3d18a137`（不在本仓库，通过 `[patch]` 引用，§4.3 / §6.2）。
- **环境变量 `MAKEPAD=ohos_sim`**（模拟器必需，§4.4）：已内置在 `ohos.sh`。

---

## 9. 故障排查

| 现象 | 原因 / 处理 |
|------|-------------|
| `'stdlib.h' file not found`（aws-lc-sys） | 没设 `BINDGEN_EXTRA_CLANG_ARGS_*` sysroot（§4.1）。用 `ohos.sh` 会自动设。 |
| `wayland-sys ... pkg-config ... cross-compilation` | rfd 没排除 ohos（§4.2）。 |
| `cannot find linux_video_playback` / `no field opengl_cx` / `-lxkbcommon` / `-lssl`/`-lcrypto` | makepad 4 个补丁没生效（§4.3）。确认已打补丁，并 `cargo clean -p makepad-platform -p makepad-network --target aarch64-unknown-linux-ohos` 后重编。 |
| 改了 makepad 却没生效 | git 依赖按 commit 缓存，需 `cargo clean -p <crate>`（§6.2）。 |
| 图标/头像是黑块 | 没设 `MAKEPAD=ohos_sim`，且改后要 `cargo clean -p makepad-platform`（§4.4）。 |
| `failed to generate signed hap package` | HAP 没签名。跑 `ohos.sh sign`，或在 build-profile.json5 配 signingConfigs（§5）。 |
| `verify certificate chain failed`（签名时） | app leaf 证书用错了（用了自签的）。要用模板里内嵌的 CA 签发 leaf（§5 第 1 步）。 |
| `bm install` 报设备不匹配 | profile 的 `device-ids` 没含当前模拟器 UDID。`ohos.sh sign` 会自动取 `hdc shell bm get --udid`。 |
| 有文字但不显示 | §6.1，模拟器浮点纹理限制，先真机验证。 |
| 真机上图标/文字异常或很慢 | 误用了 `ohos_sim` 编真机包。真机要 `MAKEPAD= harmony/ohos.sh run`（§2.1）；脚本检测到不匹配会告警。 |
| 真机 `bm install` 报签名/证书不受信任 | 真机不认 OpenHarmony 调试证书。用 DevEco 自动签名（华为账号）+ 开发者模式 + 设备注册（§2.1）。 |

---

## 10. 后续 TODO

- [ ] §6.1 文字渲染：真机验证 / 或 makepad 加 OHOS 的 MSDF 回退。
- [ ] §6.2 makepad 补丁持久化：上游 PR 或本地 `[patch]`。
- [ ] 出一版 `--release` 正式包（体积/性能）。
- [ ] 头像图片加载在弱网/离线下的表现（模拟器网络慢）。
