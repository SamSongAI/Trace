window.TRACE_SITE = {
  product: {
    name: "Trace",
    tagline: "Thought is leverage, Leave a trace.",
    summary: "Trace 把一闪而过的想法变成可检索、可执行、可路由的 Markdown 痕迹。它是工作流里最薄的一层输入接口，而不是另一个封闭笔记箱。"
  },
  current: {
    version: "1.0.3",
    releasedAt: "2026-04-14",
    releaseTitle: "Active Space Fix",
    notes: [
      "修复了在全屏或非桌面界面下，通过快捷键唤起后捕获面板没有出现在当前界面的问题。",
      "暗色主题输入框占位文字提亮，保证界面可读性并恢复测试通过。",
      "更新 macOS 下载包与站点版本信息，保持应用内版本、下载页和 GitHub Release 一致。"
    ],
    platforms: {
      macos: {
        status: "available",
        label: "Available",
        url: "./downloads/Trace.dmg",
        sha256: "fb708fe332075ce199b8f86336eb65ed767244a8d334a4f81e041f64332891f7",
        size: "612 KB",
        architecture: "Apple Silicon",
        minOS: "macOS 13+"
      },
      windows: {
        status: "planned",
        label: "Waitlist",
        url: "",
        waitlistUrl: "mailto:sam@sotasync.com?subject=Trace%20Windows%20Beta",
        sha256: "pending",
        size: "--",
        architecture: "x64",
        minOS: "Windows 10+"
      }
    }
  },
  history: [
    {
      version: "1.0.3",
      releasedAt: "2026-04-14",
      title: "Active Space Fix",
      highlights: [
        "修复全屏或非桌面界面下，捕获面板未显示在当前界面的问题",
        "提亮暗色主题输入框占位文字，恢复可读性和测试稳定性",
        "对齐应用版本、下载包和站点发布信息"
      ]
    },
    {
      version: "0.1.0",
      releasedAt: "2026-02-28",
      title: "Foundation Release",
      highlights: [
        "建立 Trace 的基础捕捉层：全局唤起、Markdown 落盘、双写入路径",
        "默认结构升级为 5 个模块化痕迹槽位",
        "设置页和桌面界面的层级统一",
        "中文输入法组合态下的保存清空逻辑修复"
      ]
    }
  ],
  roadmap: [
    {
      quarter: "2026 Q1",
      theme: "Reliable Capture Surface",
      items: [
        "稳定全局唤起、写入、校验和下载链路",
        "让每条 Trace 都以纯 Markdown、安全落盘",
        "补齐发布、签名和可验证分发流程"
      ]
    },
    {
      quarter: "2026 Q2",
      theme: "Cross-Platform Trace",
      items: [
        "推进 Windows 的全局热键、浮窗和托盘体验",
        "对齐 macOS 的 Daily / Inbox 写入一致性",
        "开放 Windows 安装包公测"
      ]
    },
    {
      quarter: "2026 Q3",
      theme: "Trace Routing",
      items: [
        "引入更强的规则、多 Vault 和自动路由能力",
        "让 Agent 和编辑器更容易消费这些 Trace",
        "建立更成熟的公开发布与反馈闭环"
      ]
    }
  ],
  pricing: [
    {
      name: "Founding Release",
      price: "¥199",
      period: "限时窗口",
      description: "尽早接入 Trace，把瞬时想法沉淀进你的 Markdown 工作流。",
      features: [
        "Trace macOS 正式版",
        "0.x 阶段持续更新",
        "优先反馈与迭代通道"
      ],
      cta: "申请 Founding Access",
      ctaUrl: "mailto:sam@sotasync.com?subject=Trace%20%E6%97%A9%E9%B8%9F%E8%B4%AD%E4%B9%B0"
    },
    {
      name: "Stable License",
      price: "¥399",
      period: "常规价",
      description: "面向稳定发布后的常规使用者，保持开放文件格式和本地优先前提不变。",
      features: [
        "Trace macOS 正式版",
        "后续稳定迭代更新",
        "适合长期使用的个人工作流"
      ],
      cta: "加入 Stable 候补",
      ctaUrl: "mailto:sam@sotasync.com?subject=Trace%20%E5%B8%B8%E8%A7%84%E8%B4%AD%E4%B9%B0"
    },
    {
      name: "Windows Waitlist",
      price: "¥0",
      period: "候补阶段",
      description: "Windows 客户端仍处于 MVP 打磨期，先开放候补，优先验证跨平台 Trace 体验。",
      features: [
        "核心写入链路已完成",
        "系统级热键与托盘能力开发中",
        "开放时将同步官网下载入口"
      ],
      cta: "加入 Windows 候补",
      ctaUrl: "mailto:sam@sotasync.com?subject=Trace%20Windows%20Beta"
    }
  ],
  faq: [
    {
      question: "为什么叫 Trace？",
      answer: "因为 Trace 关注的不是“再记一条笔记”，而是让每个稍纵即逝的念头都留下可以回溯、搜索和继续执行的痕迹。它更像工作流里的留痕层。"
    },
    {
      question: "Trace 是不是又一个笔记 App？",
      answer: "不是。Trace 只做最薄的一层输入：快速捕捉、稳定落盘、保留上下文。整理、链接、改写、分发都应该留给你现有的 Markdown 工具链。"
    },
    {
      question: "这些 Trace 只能在 Obsidian 里用吗？",
      answer: "不是。Trace 面向 Markdown。Obsidian 是高契合场景，但 VS Code、Cursor、CloudCode 或你自己的 Agent 规则都可以直接处理这些文件。"
    },
    {
      question: "为什么默认先写 Daily，而不是所有内容都拆成单文件？",
      answer: "因为很多念头在刚出现时还不值得立刻建完整结构。Trace 默认先把它们稳定留在 Daily，但也支持一键切到 Inbox 文档模式。"
    },
    {
      question: "Windows 版本什么时候可用？",
      answer: "当前已开放候补。Windows 核心写入链路已完成，待系统级热键、托盘与安装签名完成后切换为正式下载。"
    },
    {
      question: "Trace 会不会把 AI 强耦合进客户端？",
      answer: "不会。Trace 保持客户端极简，把数据以开放 Markdown 形式留在你的文件系统里。AI 更适合在外部作为处理这些 Trace 的执行层。"
    }
  ]
};
