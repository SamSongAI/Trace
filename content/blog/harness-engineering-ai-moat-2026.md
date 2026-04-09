---
title: Agent 工程最新共识：模型在卷，应用在死，Harness 是新护城河
slug: harness-engineering-ai-moat-2026
published_at: '2026-03-17'
status: published
tags:
- Harness
- Agent Engineering
- Context Engineering
excerpt: 模型还在以 6 个月的周期内卷，应用层却在同质化和脆弱工作流里快速死亡。真正能把 Agent 拉进生产环境的，不是再包一层应用，而是 Harness 这一层工程基础设施。
seo_description: 当模型迭代越来越快、应用层价值越来越薄，真正的新护城河转向 Harness：上下文工程、护栏系统和长程任务生命周期编排。
---

模型可以 6 个月迭代一次。Harness 需要系统性的、长时间的打磨。真正的护城河不在模型层，而在 Harness 层。

最近因为具体的业务需求，我需要在扣子 Coze 上落地几个 Workflow 和 Agent。

越做体感越差，都快成了 Coze 和豆包的头号黑粉。

为了捏合业务逻辑，我只能去写极度冗长的系统提示词，编排节点受限的 Workflow，外加一套精准切片后的 RAG。然后祈祷豆包能老老实实遵循指令，结果不出所料，各种意外如期而至。

当生产环境的真实 Query 大量涌入，盯着 Coze 的日志，我不得不承认一个去年就认清的事实：**工作流这种范式，在复杂真实场景下脆弱得可怜。**

这种纯靠提示词和手动拉线编排业务逻辑的基建，还停留在 2024 年的“马车时代”。

痛感来自于强烈的对比。

我每天同时也在高频使用 Claude Code，也养着自己的 OpenClaw。

这是完整的 ReAct 范式：有沙箱终端、文件系统、Bash、Plan，以及按需加载的 Skill。

最致命的是，它有一个完整的 Loop。它能自己读旧文件、写测试、跑并行审查，自己决定下一步。

而且外面已经进化到了直接调用各种 Skill、API、CLI 来自动驾驶业务逻辑的阶段，而大部分 Agent 开发平台还在让你手工拖拉拽。

范式断层极其严重。

同样是 Agent 时代，同一个底层模型，你在扣子上搭的智能体，和 Claude Code 这种范式下运行的 Agent，根本不是一个物种。能力差了不止一个量级。

差在哪？不是模型。就算你强行通过开发插件接入当前最 SOTA 的模型，它也还是个 Chatbot，成不了能自主执行长程任务的生产系统。

直到最近 Agent 领域出现了一个极其精准的概念，直接解释了这种范式级别的差异。

这个概念，叫做 Harness。

## 什么是 Harness

先说清楚它不是什么。

Harness 不是 Agent 本身，不是模型，不是 Prompt，更不是套个壳的工具链。

Geoffrey Huntley 的定义最干净：“Agent Harness 是围绕语言模型的编排层，负责 prompt 构造、tool 执行、审计检查、循环控制。如果模型是推理引擎，Harness 就是让它安全、可重复、生产可用的基础设施。”

一个类比。模型是发动机。Harness 是整台车：底盘、变速箱、刹车系统、仪表盘、导航。发动机再猛，没有这些东西你上不了路。

这个概念不新。2025 年年底，Anthropic 就在官方工程博客发过《Effective Harnesses for Long-Running Agents》，讨论如何为长时间运行的 Agent 构建约束系统。Anthropic 是最早把这件事工程化的。

但真正让 Harness 被广泛讨论的，是 OpenAI 的一个实验。

2026 年 2 月，OpenAI 发了《Harness engineering: leveraging Codex in an agent-first world》。

团队从空仓库起步，用 Codex Agent 交付了一个百万行代码的生产系统。人类全程没有手写一行代码。他们做了一个故意的选择：禁止人写代码，逼出来一套让 Agent 自主运行的基础设施。

他们发现：早期进展比预期慢，不是模型不行，而是环境的“欠定义”（underspecified），Agent 缺少推进高级目标的工具、抽象和内部结构。

> the environment was underspecified — the agent lacked the tools, abstractions, and internal structure required to make progress toward high-level goals

**环境没搭好，模型再强也白搭。搭好基建之后，生产力直接起飞。**

这个实验的价值不在于“3 个工程师驱动 Agent 写了百万行代码”这个数字，而是证明了一件事：**Agent 的天花板由模型决定，但下限和真正的可用性，全靠 Harness 兜底。**

## Harness 的三层结构

把一个虚无缥缈的大概念，变成具体的、可评估的工程标准。

Martin Fowler 在 2026 年 2 月给出了一个很好的标尺，专门发文拆解了 Harness 的架构。

他把组件分成三层。这个框架值得记住，因为它是判断一个 Agent 系统是否生产就绪的标尺，也是我目前审视所有 Agent 架构的准星。

### 第一层：Context Engineering（上下文工程）

持久化文件、跨会话记忆、子 Agent 协调、知识注入机制。

关键词是跨会话。不是一次对话结束就失忆，而是 Agent 能记住多天、多 Agent 的工作流。OpenAI 实验中就用持久化文件让 Agent 跨会话延续上下文。

这也是为什么“上下文工程”成了去年最热的 AI 技术概念之一。它不只是“怎么写 Prompt”，而是“怎么设计 Agent 的记忆系统”。

Skill 属于这一层。Anthropic 2025 年 12 月把 Agent Skills 发布为开放标准，2026 年 3 月，同一个 `SKILL.md` 在 Claude Code、Codex CLI、Cursor、Gemini CLI、GitHub Copilot 里通用。

Skill 是结构化的领域知识，Agent 按需加载，不用时不占上下文。它解决了 RAG 解决不了的问题：同时保留知识的结构和执行逻辑，而不只是向量检索碎片。

### 第二层：Quality & Guardrails（质量与护栏）

Linter 规则、架构约束、自动测试循环、政策检查、人类干预点。

这一层决定 Agent 输出的下限。没有护栏的 Agent 是赌博，有时候惊艳，有时候一塌糊涂。Harness 里的质量层，把“有时候好”变成“稳定好”。

Stripe 和 FairMindAI 已经把这一层做成了咨询框架：72 个评估标准、5 个成熟度等级。企业可以量自己的 Agent 系统到底有多成熟。

### 第三层：Orchestration & Lifecycle（编排与生命周期）

反馈循环、模型 fallback、自动重试、CI/CD 集成、状态持久化、子 Agent 管理。

让 Agent 从“跑通一次”变成“一直在线”。模型被限流时自动切备用模型，任务中途失败时从断点恢复，多个 Agent 协同时有明确的协调机制。

Martin Fowler 的判断是：人类不再写代码，而是设计 Harness 来约束和放大 Agent。人类的角色从执行者变成了架构师。

## 谁已经在做

看看真正在生产环境里跑的系统。

### Claude Code

我每天深度在用。它的实际能力范围早已经脱离了 Coding 领域，而是直接进入我每天的 CoWork。

这就是一个完整的 Harness：有终端、有 Loop、有按需加载的 Skill。

我让它升级我的 Skill，它自己读旧文件、分析问题、设计方案、写新版本、创建参考文件、写测试、跑并行测试、审查质量、打包交付。

整个过程 Agent 自己决定下一步做什么，我只在关键节点确认方向。这不是 Chatbot 能做的事。**这是一个有 Harness 的 Agent 在做事。**

### OpenClaw

这根本不是什么企业流水线上的编排工具，而是 Peter Steinberger 掀起的一场直接干掉传统 App 的社会实验。

作为今年爆发最速的开源项目之一，它的 Harness 逻辑极度硬核且回归了第一性原理：Local-first（本地优先）。

它直接跑在你的本机或 VPS 上，把飞书、电报、Discord 全变成了远程使用终端。

当个人 Agent 有了这种级别的本地 Harness 兜底，执行力的杠杆被无限放大，80% 的工具类 App 也几乎彻底失去了存在的理由。

### Skill 开放标准

别把 Skill 肤浅地理解为几段存在 Markdown 里的 Prompt 集合。2026 年初 Anthropic 推出的 Agent Skills 开放标准，本质上是解决 Context 污染的底层基建。

它用一套基于 `SKILL.md` 的渐进式暴露机制，彻底改变了给模型喂知识的方式：第一层路由标记常驻系统提示词，第二层核心逻辑按需触发，第三层冗长的参考文件让 Agent 现查现用。不用时不占注意力，触发时瞬间挂载。

巨头们已经开始基于这套标准交卷了。Vercel 官方直接下场，把团队 10 年踩坑积累的 50 多条 React 性能优化规则，硬编码成了一个标准的 Agent Skill。就连国内的百度，都已经把自己全面 Skill 化了，在前沿范式对齐这块也算走在其他大厂前面。

这些不是实验室项目。是正在生产环境里跑的系统。

## 护城河在哪

> The new moat is infrastructure that maintains coherence. You can train a better model in 6 months. Building a harness that handles multi-day workstreams takes systematic iteration.

模型可以 6 个月迭代一次。Harness 需要系统性的、长时间的打磨。

这意味着真正的护城河不在模型层，而在 Harness 层。具体来说：

**Skill 质量。** 你的行业 know-how 编码得有多深？你的 SOP 有多精细？你的上下文工程做得有多好？一个好的 Skill 需要结构化领域指令、反模式清单、质量检查标准、风格系统、可执行脚本、组件库。每一层都是从实践中打磨出来的。

**私有 Context。** 你独有的客户流和业务数据，持续喂给 Skill，让系统越跑越收敛。

**场景理解。** 同一个“合同审查”任务，律所的需求和电商的需求完全不同。谁把细分场景做到极致，谁就赢。

这三样东西，没有一样是模型提供商能替你做的，也没有一样是 Agent 平台能替你做的。

## 这件事跟你有什么关系

如果你在做 AI 相关的产品、服务或创业，有一个判断需要现在就做：

你的竞争力到底在哪一层？

在模型层？6 个月一次大迭代。你站在流沙上。模型能力延长线等于创业者的达摩克利斯之剑。

在应用层？做一个 Chatbot 套壳，调个 API，封装几个工具包装一下？这个门槛和价值都在快速归零。

真正的壁垒在 Harness 层。把你的行业认知和场景理解打成 Skill，把你的 SOP 变成护栏。这条路很重、很苦，需要极大的专注力去反复打磨。但好处是：别人想抄，只能把你的泥坑重新踩一遍。

拿我自己的体感来说：在具体的 Project 中沉淀下来的高密度个人 Skill，我绝对不会轻易交出去。那是我的私人杠杆。还有一部分经过检验、能直接解决企业长程任务痛点的 Skill，才是用来换取经济价值的筹码。

Philipp Schmid 说得更直接：“没有 Harness，用户体验可能落后于模型的潜力。”

当执行能力被基础设施拉平，模型一样强，工具一样多，API 一样便宜，剩下的差异就是 Harness 的质量。

就是你的数据清洗有多深、上下文工程做得有多好、你的 Skill 迭代了多少次、你的护栏搭得有多稳。

如果你没有构建自己的 Harness，那么 Agent 时代，你的技术债迟早要用其他资源代偿，时间和 Token 都很宝贵。

对个人和对公司来说都是如此，因为对 Agent 来说，你就是他们的 CEO。

## 走向新世界

2026 年开始，Agent 世界已经彻底变了。我们的核心动作，正在从“管理人类执行者”，完全转向“管理 AI Agents”。

而你绝不可能靠几个拖拉拽的 Workflow 和简陋的系统提示词，去管理一个能在生产环境里打仗的 Agent 团队。

去搭你的 Harness，去积累你的时间壁垒。
