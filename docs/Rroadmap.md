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

### Phase 4 — 体验优化 (Week 8)

**目标:** CLI 体验打磨、分发渠道建设。

| 任务 | 产出 |
|------|------|
| Shell 补全生成 (`--generate-completion bash/zsh/fish`) | 补全脚本 |
| Man page 生成 | `man fit-editor` |
| 彩色/结构化输出 (colored, tabled) | 彩色表格输出 |
| 进度指示器 (indicatif) | 批量操作进度 |
| Homebrew Formula | `brew install fit-editor` |
| GitHub Release 预编译二进制 | macOS/Linux/Windows |
| crates.io 发布 | `cargo install fit-editor` |

### 版本规划

| 版本 | 对应 Phase | 核心特性 |
|------|-----------|----------|
| **v0.1.0** | Phase 0+1 | 查看、导出 (JSON/CSV) |
| **v0.2.0** | Phase 2 | 编码、编辑、hexdump |
| **v0.3.0** | Phase 3 | 合并、拆分、diff、GPX |
| **v1.0.0** | Phase 4 | 稳定 API、包管理分发 |

### 技术债务 & 未来展望

| 主题 | 说明 |
|------|------|
| 流式处理 | 大文件支持，避免全量内存加载 |
| 插件系统 | 自定义导出格式 |
| 交互模式 | TUI 界面 (ratatui) 浏览 FIT 文件 |
| 压缩时间戳编码 | SDK 支持后启用输出优化 |
| 自定义 Profile | 加载第三方 Profile.xlsx |
