# Product Requirements Document (PRD)

## fit-editor — 纯 Rust FIT 文件编解码 CLI 工具

### 1. 产品概述

**产品名称:** `fit-editor`
**产品类型:** 命令行工具 (CLI)
**目标用户:** 运动数据开发者、Garmin/FIT 设备用户、运动数据分析爱好者
**核心价值:** 提供一个高性能、纯 Rust 实现的 FIT 文件查看与编辑工具，无需依赖 Garmin SDK 或其他外部工具。

### 2. 背景

FIT (Flexible and Interoperable Data Transfer) 是 Garmin 主导的二进制数据格式，广泛用于运动手表、自行车码表、健康设备等场景。目前主流的 FIT 操作工具（如 Garmin FIT SDK 的 C 版本、fitdecode (Python)、FitFileViewer (Swift)）要么需要 C 依赖，要么功能有限。

本项目基于纯 Rust 实现的 [`fit-sdk-rust`](https://github.com/Chen-Lim/fit-sdk-rust) SDK，构建一个全功能 CLI 工具。

### 3. 目标用户画像

| 用户角色 | 需求 |
|---------|------|
| **运动 App 开发者** | 查看 FIT 文件结构、调试编解码问题、验证生成的 FIT 文件 |
| **数据分析师** | 提取 FIT 数据为 CSV/JSON，进行数据分析 |
| **Garmin 用户** | 查看运动记录、修改活动元数据、合并/拆分活动文件 |
| **固件/嵌入式工程师** | 验证设备生成的 FIT 文件格式兼容性 |

### 4. 功能需求

#### 4.1 P0 — 核心功能 (MVP)

| 功能 | 命令 | 说明 |
|------|------|------|
| 文件验证 | `fit-editor validate <file>` | 校验 FIT 文件的 CRC 完整性、协议版本、签名 |
| 文件信息 | `fit-editor info <file>` | 显示文件头信息（协议版本、Profile 版本、数据大小） |
| 消息查看 | `fit-editor dump <file>` | 以人类可读格式输出所有消息（支持 `--raw` 原始值模式） |
| 消息过滤 | `fit-editor dump <file> --message <type>` | 按消息类型过滤（如 `record`, `session`, `lap`） |
| JSON 导出 | `fit-editor export <file> --format json` | 将消息导出为 JSON 格式 |
| CSV 导出 | `fit-editor export <file> --format csv` | 将 Record 消息导出为 CSV（含表头） |
| 编码写回 | `fit-editor encode <json-file> -o <output.fit>` | 从 JSON 重新编码为 FIT 文件 |

#### 4.2 P1 — 增强功能

| 功能 | 命令 | 说明 |
|------|------|------|
| 字段修改 | `fit-editor edit <file> --set session.total_distance=5000.0` | 修改指定消息的字段值 |
| 消息删除 | `fit-editor edit <file> --remove-message <type>` | 删除指定类型的消息 |
| 消息追加 | `fit-editor edit <file> --append <json-file>` | 向现有 FIT 文件追加新消息 |
| 文件合并 | `fit-editor merge <file1> <file2> -o <output>` | 合并多个 FIT 文件的时间线 |
| 文件拆分 | `fit-editor split <file> --at <timestamp>` | 在指定时间点拆分 FIT 文件 |
| Hex 查看 | `fit-editor hexdump <file>` | 输出 FIT 文件的十六进制内容 |

#### 4.3 P2 — 高级功能

| 功能 | 命令 | 说明 |
|------|------|------|
| 差异对比 | `fit-editor diff <file1> <file2>` | 对比两个 FIT 文件的差异 |
| 批量处理 | `fit-editor batch <glob> -- <command>` | 对匹配的文件批量执行命令 |
| 统计摘要 | `fit-editor summary <file>` | 显示运动活动摘要（距离、时间、心率均值等） |
| 坐标转换 | `fit-editor export <file> --format gpx` | 导出为 GPX 格式 |
| Schema 校验 | `fit-editor validate <file> --strict` | 严格模式校验（字段范围、必填字段检查） |

#### 4.4 0.1.0 Known issues (2026-05-13 strict review)

代码已实现 4.1–4.3 的全部命令，但 strict review 报告（见 [`../Report.html`](../Report.html)）发现以下行为偏离 PRD：

| ID | 命令 | 行为偏差 | 修复阶段 |
|----|------|----------|----------|
| C1 | `edit` / `encode` | enum 字段每次解析泄漏 `String` 缓冲（`Box::leak`） | Phase 4.6 |
| C2 | `export --format gpx` | 室内活动（无 GPS）的 record 被写为 `lat=0,lon=0` 假轨迹点 | Phase 4.6 |
| C3 | `merge` | 输出包含多份 `file_id` / `file_creator`；timestamp 排序键失效（实际按文件顺序拼接） | Phase 4.6 |
| H1 | `hexdump --annotate` | 截断输入触发 array index panic | Phase 4.6 |
| H2 | `validate` | 仅校验 CRC，不检查 decoder-level 错误，可能对损坏文件返回 "valid" | Phase 4.6 |
| H3 | 全局 | `--quiet` 已声明但未实现，开关无效 | Phase 4.6 |
| H4 | 所有命令 | 无文件大小上限，恶意巨型输入会 OOM | Phase 4.6 |

详见 [Rroadmap.md `Phase 4.6`](./Rroadmap.md) 与 [Report.html](../Report.html) 全文。

### 5. 非功能需求

| 维度 | 要求 |
|------|------|
| **性能** | 单个 FIT 文件解码 < 100ms（典型 5MB Activity 文件） |
| **跨平台** | 支持 macOS (ARM64/x86_64)、Linux (x86_64/ARM64)、Windows (x86_64) |
| **可靠性** | 解码器不应 panic，所有错误通过 `FitError` 类型化传播。⚠️ 当前 0.1.0-alpha 在 `hexdump --annotate` 上仍存在一处 panic 路径（H1），将在 Phase 4.6 修复 |
| **CLI 体验** | 彩色输出、进度条（批量操作）、shell 补全、TTY 自动检测 |
| **无外部依赖** | 不依赖 libfit、Garmin SDK 或任何 C 库 |
| **安装** | 通过 `cargo install` 或预编译二进制分发（计划，阻塞于 Phase 4.6） |
| **输入大小** | 默认拒绝 > 256 MiB 的输入（计划，Phase 4.6），可通过 `--max-file-size` 覆盖 |

### 6. 技术约束

- **FIT 协议版本:** v2.0 (protocol_version = 0x20), Profile v21.200
- **SDK 限制:** 当前 SDK 不压缩时间戳编码（输出始终使用显式时间戳），round-trip 语义等价但不字节级相同
- **Rust 版本:** MSRV 1.75+（与 SDK 一致）
- **许可证:** GPL-3.0（与 `Cargo.toml` 一致；PRD 旧版误写为 Apache-2.0，已更正）

### 7. 成功指标

| 指标 | 目标 |
|------|------|
| validate 命令正确识别损坏文件 | 100% |
| dump/decode 无 panic | 100% |
| decode → encode → decode 往返一致性 | > 99.9% 字段值匹配 |
| 典型 Activity 文件处理时间 | < 100ms |
| 二进制大小 (release) | < 5MB |

### 8. 里程碑

参见 [Rroadmap.md](./Rroadmap.md)。

### 9. 竞品分析

| 工具 | 语言 | 优势 | 劣势 |
|------|------|------|------|
| **FitCSVTool.jar** (Garmin) | Java | 官方工具 | 需要 JVM，仅 CSV 互转 |
| **fitdecode** | Python | 易用 | 只读解码，无编码能力 |
| **fit-sdk-rust** | Rust | 纯 Rust，编解码 | SDK 级别，非用户工具 |
| **GoldenCheetah** | C++ | 功能丰富 | 重量级 GUI，非 CLI |
| **fit-editor (本项目)** | Rust | 纯 Rust，CLI，编解码全 | — |

### 10. 风险与缓解

| 风险 | 概率 | 影响 | 缓解措施 |
|------|------|------|----------|
| SDK Profile 数据不完整 | 中 | 高 | 预留 fallback 到 raw 值显示 |
| 大文件内存压力 | 中 | 高 | Phase 4.6 引入 `--max-file-size` guard，长期改流式 IO |
| SDK `Value::Enum(&'static str)` API 强制消费者 leak 内存 | 高 | 高 | Phase 4.6 与 fit-sdk-rust 协调改为 `Arc<str>` |
| Garmin 新协议版本不兼容 | 低 | 高 | SDK 层抽象，版本检查 |
| 压缩时间戳 round-trip 文件增大 | 高 | 低 | 文档说明语义等价性 |
| 二进制格式 fuzz 未覆盖，潜在 panic 路径 | 高 | 中 | Phase 4.6 引入 `cargo fuzz` target |
