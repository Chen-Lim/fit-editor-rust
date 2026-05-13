# Roadmap

## fit-editor 开发路线图

### Phase 0 — 项目搭建 (Week 1)

**目标:** 建立可编译的项目骨架，集成 fit-sdk-rust SDK。

| 任务 | 产出 | 优先级 |
|------|------|--------|
| `cargo init` 创建二进制 crate | `src/main.rs`, `Cargo.toml` | P0 |
| 添加 `fit-sdk-rust` 为 path 依赖 | `Cargo.toml` 引用 `../fit-sdk-rust` 或 git | P0 |
| 引入 CLI 框架 (clap) | 子命令骨架 | P0 |
| 引入序列化 (serde + serde_json) | JSON 导出支持 | P0 |
| 引入 CSV writer (csv crate) | CSV 导出支持 | P0 |
| 基础错误处理 (anyhow/thiserror) | 统一错误输出 | P0 |
| CI 配置 (GitHub Actions) | lint + test + build matrix | P1 |

### Phase 1 — MVP: 查看与导出 (Week 2-3)

**目标:** 实现核心的查看和导出功能，用户可以打开任意 FIT 文件并理解其内容。

| 命令 | 功能 | 交付 |
|------|------|------|
| `fit-editor validate <file>` | CRC 完整性校验 | 通过/不通过 + 错误详情 |
| `fit-editor info <file>` | 文件头元数据展示 | 协议/Profile 版本、数据大小、消息统计 |
| `fit-editor dump <file>` | 人类可读消息输出 | 分层缩进显示，支持 `--raw` |
| `fit-editor dump --message <type>` | 消息过滤 | 仅输出指定类型消息 |
| `fit-editor export --format json` | JSON 导出 | 完整消息数组 |
| `fit-editor export --format csv` | CSV 导出 | Record 消息的字段列 |

**验收标准:**
- [ ] 能解码 `Activity.fit` 测试文件并显示所有消息
- [ ] JSON 导出可被 `jq` 正确解析
- [ ] CSV 导出可被 Excel/Pandas 正确读取
- [ ] validate 能正确检测损坏文件

### Phase 2 — 编码与编辑 (Week 4-5)

**目标:** 实现 FIT 文件的创建和修改能力，完成编解码闭环。

| 命令 | 功能 | 交付 |
|------|------|------|
| `fit-editor encode <json> -o <fit>` | JSON → FIT 编码 | 通过 CRC 校验的合法 FIT 文件 |
| `fit-editor edit --set <field>=<value>` | 字段修改 | 修改后的 FIT 文件 |
| `fit-editor edit --remove-message <type>` | 消息删除 | 指定消息类型被移除 |
| `fit-editor hexdump <file>` | 十六进制查看 | 类 `xxd` 输出，高亮消息边界 |

**验收标准:**
- [ ] decode → encode → decode 往返字段值一致
- [ ] 编码输出通过 `fit-editor validate` 校验
- [ ] edit 操作后 session summary 字段自动更新

### Phase 3 — 高级操作 (Week 6-7)

**目标:** 多文件操作、格式转换、批量处理。

| 命令 | 功能 | 交付 |
|------|------|------|
| `fit-editor merge <f1> <f2>` | 文件合并 | 时间线排序、local_mesg 重映射 |
| `fit-editor split <file> --at <ts>` | 文件拆分 | 两个独立 FIT 文件 |
| `fit-editor diff <f1> <f2>` | 差异对比 | 结构化差异输出 |
| `fit-editor summary <file>` | 活动摘要 | 距离/时间/心率/速度统计 |
| `fit-editor export --format gpx` | GPX 导出 | 仅 record 消息的轨迹点 |
| `fit-editor batch <glob> -- <cmd>` | 批量处理 | 并行执行，进度条 |

### Phase 4 — 体验优化 (Week 8) ✅ 已交付

**目标:** CLI 体验打磨。

| 任务 | 状态 |
|------|------|
| Shell 补全生成 (bash/zsh/fish/powershell/elvish) | ✅ `completion` 子命令 |
| Man page 生成 | ✅ `build.rs` + `clap_mangen` |
| 彩色/结构化输出 (colored, tabled) | ✅ |
| 进度指示器 (indicatif) | ✅ `batch` 命令 |
| TTY 自动检测 + `--no-color` | ✅ `main.rs:13-16` |

### Phase 4.6 — Post-review hardening (阻塞 v0.1.0 release)

**触发:** 2026-05-13 strict code review，详见 [`../Report.html`](../Report.html)。

**目标:** 在向 crates.io / Homebrew 发布之前，修掉一份 review 报告里列出的 3 个 CRITICAL、4 个 HIGH、6 个 MEDIUM 问题，并补齐 lib.rs / fuzz / CI 基础设施。在这些工作完成前，**v0.1.0 不应被视为可发布版本**。

#### Required fixes (FIX)

| ID | Severity | 任务 | 引用 |
|----|----------|------|------|
| C1 | CRITICAL | 替换两处 `Box::leak()`；协调 `fit-sdk-rust` 的 `Value::Enum` API 改为 `Arc<str>` / `Cow<'static, str>` / `String` | [edit.rs:167](../src/commands/edit.rs), [encode.rs:123](../src/commands/encode.rs) |
| C2 | CRITICAL | GPX 导出对无 GPS 的 record `continue`，移除 fallback 到 (0,0) | [export.rs:205-216](../src/commands/export.rs) |
| C3 | CRITICAL | `merge` 去重后续文件的 `file_id` / `file_creator` / `device_info`；用 `Value::DateTime` 提取 timestamp | [merge.rs:31-35, 53-55](../src/commands/merge.rs) |
| H1 | HIGH | `hexdump::estimate_definition_size` 边界检查改为 `data.len() < 6` | [hexdump.rs:122-128](../src/commands/hexdump.rs) |
| H2 | HIGH | `validate` 必须调用 `Decoder::read_all` 并报告 errors；非零 decoder error 影响 exit code（建议 exit 2） | [validate.rs](../src/commands/validate.rs) |
| H3 | HIGH | 实现 `--quiet` 跨所有子命令；或从 `Cli` struct 删除该 flag | [cli.rs:11-13](../src/cli.rs) |
| H4 | HIGH | 增加 `--max-file-size` guard（默认 256 MiB）覆盖所有 `fs::read` / `fs::read_to_string` | all command entrypoints |
| M1 | MEDIUM | `ProgressStyle::with_template(...).unwrap()` → `.expect("...")` | [batch.rs:33](../src/commands/batch.rs) |
| M2 | MEDIUM | `diff` 主循环改为 `itertools::zip_longest`，移除 `unreachable!()` | [diff.rs:67](../src/commands/diff.rs) |
| M3 | MEDIUM | encode 数值转换优先 `as_u64`/`as_i64`，显式范围检查；超界返回 `Err` 而非 `Value::Invalid` | [encode.rs:97-112](../src/commands/encode.rs) |
| M4 | MEDIUM | `batch` 改为返回 `Err(CliError::BatchPartialFailure)`；移除 `process::exit(1)` | [batch.rs:103-106](../src/commands/batch.rs) |
| M5 | MEDIUM | 引入 `CliError::BadUsage(String)`；迁移 batch/merge 业务校验错误 | [error.rs](../src/error.rs) |
| M6 | MEDIUM | `split` 输出 prefix 拒绝 `../` 段；canonicalize 父目录 | [split.rs:48-58](../src/commands/split.rs) |
| L4 | LOW | 移除未使用的 `anyhow` 依赖 | [Cargo.toml:24](../Cargo.toml) |

#### Required additions (ADD)

| ID | 任务 |
|----|------|
| A1 | 引入 `src/lib.rs`；把 `commands::*` 改为 lib-callable；为 `parse_set_arg` / `extract_timestamp` / `semicircles_to_degrees` / `estimate_definition_size` 等纯函数补单元测试 |
| A2 | `cargo fuzz init`；为 `fit::Decoder::read_all` 与 `fit::FileHeader::parse` 写 fuzz target；resubmit 前本地至少跑 1 小时 |
| A3 | `.github/workflows/ci.yml`：build / test / clippy `-D warnings` / `cargo fmt --check` / `cargo audit` |
| A4 | 集成测试：`merge` / `split` / `diff` / `validate` / GPX 导出 / CSV 导出（当前全部为零覆盖） |
| A5 | `#![forbid(unsafe_code)]` 加在 `src/main.rs`（或重构后的 `lib.rs`） |

#### 验收门槛

- 所有 FIX 条目合并；
- A1 + A3 + A2（至少 1h fuzz 无新发现）合并；
- A4 至少覆盖 `merge` / `split` / GPX 导出（其余可推迟到 v0.2.0，但要在 commit message 中明确承诺时间）。

### 版本规划

| 版本 | 对应 Phase | 状态 | 核心特性 |
|------|-----------|------|----------|
| **v0.1.0-alpha** | Phase 0+1+2+3+4 | ⚠️ feature-complete but **NOT release-ready** | 查看、导出、编辑、合并、拆分、diff、GPX、batch；含 3 个 CRITICAL bug，见 Phase 4.6 |
| **v0.1.0** | Phase 4.6 完成 | 🔒 阻塞 | release 候选，仅在 Phase 4.6 全部 FIX + A1/A2/A3 合并后才能 tag |
| **v0.2.0** | A4 全部 + 流式 IO + 严格模式 | 计划 | 完整 CI、测试覆盖、`Decoder::strict()` 模式 |
| **v1.0.0** | 稳定 API + 分发 | 计划 | Homebrew Formula / GitHub Release 二进制 / crates.io |

### 技术债务 & 未来展望

| 主题 | 说明 |
|------|------|
| 流式处理 | 大文件支持，避免全量内存加载 |
| 插件系统 | 自定义导出格式 |
| 交互模式 | TUI 界面 (ratatui) 浏览 FIT 文件 |
| 压缩时间戳编码 | SDK 支持后启用输出优化 |
| 自定义 Profile | 加载第三方 Profile.xlsx |
