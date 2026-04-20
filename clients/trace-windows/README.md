# Trace Windows 客户端

Trace 原生 macOS 应用的 Windows 端实现，使用 Rust + [iced](https://github.com/iced-rs/iced) + [windows-rs](https://github.com/microsoft/windows-rs) 构建。当前仓库目录处于 Phase 0（脚手架）阶段，只保证可以编译与跑 core 层测试。

> 旧版 Rust MVP 仍保留在 `clients/trace-win/`，**不要与本目录混用**。

## Workspace 结构

```
clients/trace-windows/
├── Cargo.toml              # Workspace 清单，固定所有 crate 的版本与依赖
├── rust-toolchain.toml     # 锁定 stable 工具链 + rustfmt/clippy
├── .cargo/config.toml      # Windows 目标的 target-cpu 配置
└── crates/
    ├── trace-core/         # 跨平台领域逻辑，零 Windows 依赖，可在 macOS/Linux 跑测试
    ├── trace-platform/     # Windows 系统集成，仅在 Windows 目标下链接 `windows` crate
    ├── trace-ui/           # iced UI 层，跨平台编译
    └── trace-app/          # 可执行入口（[[bin]]），Phase 0 仅打印版本
```

## 前置依赖

- Rust stable（`rust-toolchain.toml` 会自动拉取，无需手动安装特定版本）
- Windows：Visual Studio 2022 Build Tools（含 MSVC + Windows SDK）
- macOS / Linux：仅能验证 `trace-core`；`trace-platform` 与 `trace-app` 在非 Windows 目标下虽然 `cargo check` 通过，但不会被视作发布构建

## 构建命令

### macOS / Linux 开发机

```bash
cd clients/trace-windows

# 编译并测试跨平台 core 层
cargo test -p trace-core

# 静态检查整个 workspace（platform 层会退化为空 crate，不会失败）
cargo check --workspace
```

### Windows

```powershell
cd clients\trace-windows

# 完整编译（含 trace-platform 与 trace-app）
cargo build --release

# 运行所有测试
cargo test

# 运行可执行程序（目前只打印版本）
cargo run -p trace-app
```

## 后续阶段

完整的阶段拆分见仓库根目录 `docs/plans/2026-04-20-windows-client-implementation.md` 的 §6。Phase 0 只负责打地基，Phase 1 起才会填充数据模型、写入逻辑、UI 与 Windows 集成。
