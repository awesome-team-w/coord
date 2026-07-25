# coord — 多 agent 并行修改协调工具 设计文档

日期：2026-07-25
状态：已与用户确认方向，待实现计划

## 问题

多个 coding agent（或同一 agent 的多个 session）在同一代码库上并行执行不同任务时：

1. 可能同时修改同一文件，互相覆盖或产生冲突；
2. 共享同一个 git index，`git add -A` 会把别的 session 未完成的脏文件一起 commit；
3. 无从知道某个文件"正在被谁、为了什么任务"修改。

目标：在不阻塞任何 session 思考与进展的前提下，让并行修改自然不冲突，且文件归属清晰可查。

## 核心理念

**协调靠约定与自觉，不靠拦截与强制。**

- 不做文件系统监控，不做 hook 强制拦截，无常驻进程（daemon）。
- 安装时将协作规则注入 repo 的 `AGENTS.md`；agent 读到规则后，在动笔前**主动登记**（claim），完成后**主动注销**（done）。
- CLI 是共享的"任务登记簿"：记账 + 信息台，不是门卫。
- AGENTS.md 是跨 agent 事实标准（Claude Code / Codex / Cursor 均读取），因此协议天然通用；Claude Code 额外通过 skill 获得更细的工作流指导。

## 已确认的关键决策

| 决策点 | 结论 |
|---|---|
| 目标生态 | Claude Code 优先，核心与 agent 无关（纯 CLI 协议） |
| 冲突语义 | 租约 + 告知重排：后来者得到"谁/什么任务/多久了"的情报，自行决定重排、等待或 `--force` 共同编辑 |
| 租约获取 | agent 主动 `claim`（由 AGENTS.md 规则驱动），非 hook 隐式拦截 |
| 租约释放 | `task done` 主动释放；崩溃场景由惰性清理兜底 |
| commit 策略 | agent 自己 commit，CLI 提供限定范围包装（只 stage 本任务 claim 的文件） |
| 常驻进程 | 无。状态为 repo 内 SQLite 文件，短命 CLI 调用 + 文件锁保证原子性 |
| 暂存排队合入 | 明确放弃（假装写成功会让 agent 基于错误前提继续工作） |

## 架构与组件

三个交付物，一个 GitHub repo：

```
coord/
├── cli/          # Rust CLI（记账核心）
├── skill/        # Claude Code skill（工作流指导）
└── templates/    # AGENTS.md 注入区块模板
```

### 1. CLI（Rust）

状态存储：`<repo>/.agentcoord/state.db`（SQLite），`coord init` 时自动加入 `.gitignore`。所有命令为短命进程，用 SQLite 自带锁保证并发原子性。

命令面（v1 固定为这 6 个，YAGNI）：

```
coord init                        # 注入 AGENTS.md 规则区块 + 建状态库 + .gitignore
coord task start "<任务描述>"      # 注册任务 → 返回 task id（如 T12）
coord claim -t T12 <path>...      # 动笔前登记文件/目录；已被占用时返回占用情报（非硬拒绝）
coord status                      # 看板：哪个任务正在改哪些文件、改了多久、是否陈旧
coord commit -t T12 -m "<msg>"    # 只 stage 本任务 claim 过的路径，commit 并附任务信息
coord task done T12               # 释放本任务全部登记，输出交接摘要
```

**任务身份传递**：agent 的每次 shell 调用都是独立进程，无法靠环境变量或 cwd 关联任务，因此 task id 必须显式传参（`-t`）。`task start` 的输出会明确提醒 agent"后续命令请带上 -t T12"，AGENTS.md 区块与 skill 中同样强调。这也天然支持一个 session 并行推进多个任务。

**PID 探活对象**：`task start` 时从 CLI 自身向上遍历祖先进程，记录第一个长命祖先（通常即 agent 进程）作为探活对象；探测不可用时（进程树异常、容器环境）退化为纯时限判断。

`claim` 冲突时的输出示例（机器可读 + 人可读双格式）：

```
CLAIMED src/auth.ts
  by task#12 "重构登录流程" (session 48291, 8 分钟前)
建议：先处理其他文件，或 coord status 查看进展；确认可并行时用 --force 登记共同编辑（会留下记录）。
```

`--force`：允许共同编辑，登记为 co-claim，双方在 `status` 中均可见，风险自担、有迹可查。

### 2. AGENTS.md 注入区块

`coord init` 在 repo 根部 `AGENTS.md`（无则创建）插入受管区块：

```
<!-- coord:begin -->
（给 agent 的协作协议：任务开始先 task start；修改任何文件前先 claim；
被占用时依据返回情报重排子任务顺序；完成后 coord commit + task done）
<!-- coord:end -->
```

- 标记区块保证 `coord init` 幂等（重复运行只更新区块），`coord uninit`（v2 再议）可干净移除。
- 区块内容强调三条铁律：**先登记后动笔、commit 只用 coord commit、结束必须 done**。

### 3. Skill（Claude Code 增强层）

skill 内容（不是强制层，是"用得好"层）：

- 如何写让别的 session 看得懂的任务描述；
- 收到占用情报后如何把子任务重排（先做不冲突的部分）；
- 僵尸登记的识别与安全接管流程；
- 多文件任务的 claim 粒度建议（文件 vs 目录）。

## 数据模型（SQLite）

```
tasks(id, description, session_pid, started_at, finished_at NULL)
claims(task_id, path, claimed_at, released_at NULL, forced BOOL)
```

路径冲突判定：新 claim 与未释放 claim 的路径存在包含关系（文件≺目录）即为占用。

## 异常处理

- **僵尸登记**：session 崩溃不会 `task done`。任何 CLI 命令执行时惰性清理：对未完成任务做 PID 探活；PID 已死或登记超过时限（默认 2 小时，可配）→ 标记为 `stale`，`status` 中明示，他人 claim 时告知"原持有者已失联，可安全接管"。
- **AGENTS.md 被手工改动**：只认标记区块，区块外内容永不触碰；区块被删则 `init` 重新注入。
- **repo 无 git**：`coord commit` 报错并提示；其余命令正常（登记功能不依赖 git）。

## 明确不做（v1）

- 文件系统监控 / fswatch；
- hook 强制拦截（架构兼容，v2 可作 opt-in 兜底）；
- 常驻 daemon；
- 修改暂存排队与自动合入；
- 跨机器/远程协调（仅本机多 session）。

## 成功标准

1. 两个 Claude Code session 在同一 repo 并行执行不同任务，全程无文件互踩、无 commit 串包；
2. 任一时刻 `coord status` 能说清每个被登记文件的归属任务与时长；
3. agent 未安装 skill、仅凭 AGENTS.md 区块也能正确遵循协议（用 Codex/其他 agent 验证通用性）；
4. 单 session 使用时无额外负担（协议开销 ≤ 每任务 3 次 CLI 调用）。

## 测试策略

- CLI 核心（claim 冲突判定、路径包含、惰性清理、scoped commit）：Rust 单元 + 集成测试，TDD；
- 并发：多进程同时 claim 同一路径的竞争测试（必须恰好一方成功）；
- 端到端：脚本模拟两个 session 的完整协议流程；
- skill/AGENTS.md 文案：用真实双 session 手工验证遵从性。
