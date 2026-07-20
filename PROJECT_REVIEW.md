# tiez-clipboard 项目审查报告

**范围：** 仅 `tiez-clipboard`（Tauri 2 + React 桌面剪贴板）  
**审查日期：** 2026-07-16  
**目标：** 矛盾点、隐藏 bug、资源占用  
**状态：** 审查交付（未改代码）

---

## 1. 概览

| 项 | 说明 |
|----|------|
| 技术栈 | Tauri 2 + React + rusqlite |
| 平台 | 代码/版本轨支持 macOS + Windows；README badge 写 macOS-only |
| 体量 | 整体约 15GB（主要是 `src-tauri/target` debug 产物） |
| 版本源 | `versions.json`：macOS `0.1.4` / Windows `0.3.4`；`package.json` / `tauri.conf` / `Cargo.toml` 常见 `0.1.4`（构建脚本会按平台改写） |

---

## 2. 隐藏 Bug（按严重度）

### P0

#### 2.1 类型筛选参数名错误（很可能失效）

- **位置：** `src/shared/hooks/useHistoryFetch.ts`（约 L99）
- **现象：** `get_clipboard_history` 传 `content_type`；同文件搜索却用 camelCase：`searchTerm` / `tagOnly` / `noteOnly`
- **后端：** `src-tauri/src/app/commands/history_cmd.rs` 参数为 `content_type`
- **原因：** Tauri 2 默认 JS 侧用 camelCase 映射 Rust snake_case → 应传 `contentType`
- **修复：**

```ts
contentType: typeFilter || undefined
```

并补类型筛选测试。

#### 2.3 Windows 剪贴板监听几乎不可用

- **位置：** `src-tauri/src/services/clipboard_listener.rs` 非 macOS 分支
- **证据：** 仍调用 `macos_api::clipboard::get_clipboard_sequence_number()`；注释写 *Windows-specific wnd_proc path removed*
- **后果：** Windows 编译/运行监听不可靠；`windows_api` 存在但未接主监听
- **修复：** 非 macOS 接 `windows_api`，或明确 `cfg` 限制平台

#### 2.4 生产开启 DevTools

- **位置：** `src-tauri/tauri.conf.json` → `devtools: true`
- **后果：** 发布包可开调试工具，泄露内部状态
- **修复：** 仅 debug / feature flag 开启

---

### P1

#### 2.5 MQTT「重启」不真正断连

- **位置：** `src-tauri/src/services/mqtt_sub.rs` → `restart_mqtt_client`
- **问题：** 只置 `MQTT_RUNNING=false`；事件循环主要在 error / config None 时退出；配置变更可能仍挂旧 broker
- **修复：** cancel token + 强制 disconnect/join 后再起

#### 2.6 MQTT 每条消息 `std::thread::spawn`

- **位置：** `mqtt_sub.rs`（约 L469）
- **问题：** 每条消息新 OS 线程 + arboard + 入库 → 高频时线程风暴、抢剪贴板
- **修复：** 有界 worker / 串行队列

#### 2.7 Cloud Sync stop/restart 语义弱

- **位置：** `src-tauri/src/services/cloud_sync.rs`（约 3200–3360）
- **问题：**
  - `stop` 只置 cancel；外层 1s 等待环不查 cancel
  - `start` 在 `CLOUD_SYNC_TASK_ACTIVE` 已 true 时 no-op
  - `restart` ≈ 请求一次同步，不是真重启
- **修复：** 外环查 cancel；JoinHandle/AbortHandle；restart = stop(await) + start

#### 2.8 顺序粘贴失败时队列处理不完整

- **位置：** `src-tauri/src/services/paste_queue.rs`
- **问题：** `prepare_clipboard_payload` 失败后 emit `queue-item-pasted` 并 `return`，未继续后续项/未清晰回滚
- **修复：** 标记失败后继续下一项，或清空队列并统一错误事件

#### 2.9 加密队列锁失败时丢任务

- **位置：** `src-tauri/src/services/encryption_queue.rs`（约 L46–50）
- **问题：** `conn.lock()` `Err` 时 `continue`，job 不重入队 → 敏感项可能永久明文
- **修复：** poison 恢复或 requeue + 告警

#### 2.10 分页偏移与 Session 合并 skew

- **位置：** `history_cmd.rs` + `useHistoryFetch.ts`
- **问题：** offset=0 合并 session 再 truncate；前端 offset 只计 `id > 0`；边界可能漏页/重页
- **修复：** 会话与持久化分页分离，或统一 cursor

#### 2.11 预览文件 watch 线程模型重

- **位置：** `src-tauri/src/services/content_handler.rs`
- **问题：** 每 entry 一线程，2s 轮询，最长约 1h
- **修复：** 单 watcher / 线程池 + 超时回收

#### 2.12 多处非 macOS 仍直接调 `macos_api`

- **位置：** `paste_queue`、`clipboard_ops`、`window_manager` 等
- **问题：** Windows 路径可能编译失败或空操作
- **修复：** 全面 `cfg` 分流，统一 platform 抽象

---

### P2

| 项 | 说明 |
|----|------|
| 粘贴默认 `shift_insert` | 偏 Windows；macOS 更宜 `cmd_v` 类默认（见 `paste_queue.rs` / `clipboard_ops.rs`） |
| 版本多源 | `versions.json` 与 `package.json` / `tauri.conf` / `Cargo.toml` 不一致 |
| 双虚拟列表库 | `react-window` + `react-virtuoso`；实际用 Virtuoso，`resetAfterIndex` 为 no-op |
| 插件重复 | `tauri-plugin-sql` 已注册，业务实际只用 rusqlite |
| 剪贴板栈重复 | arboard + clipboard_rs + 原生 API 三套并存 |

---

## 3. 矛盾点

### 3.1 平台叙事不一致

- README Platform badge：**macOS-only**
- 同时存在：Windows 依赖、`versions.json` windows 轨道、Windows API 模块
- **影响：** 用户预期混乱；Windows 质量债被文档掩盖

### 3.2 版本源分裂

| 源 | 版本 |
|----|------|
| `versions.json` | macOS **0.1.4** / Windows **0.3.4** |
| `package.json` / `tauri.conf.json` / `Cargo.toml` | 常见 **0.1.4**（`scripts/apply-app-version.mjs` 构建时改写） |

### 3.3 架构重复 / 死代码倾向

- DB：plugin-sql 注册但 rusqlite only
- 列表：react-window 依赖在，运行时 Virtuoso
- 剪贴板：三套 API 并存

### 3.4 God 文件

| 文件 | 约行数 |
|------|--------|
| `src-tauri/src/services/cloud_sync.rs` | 3561 |
| `src-tauri/src/services/clipboard/utils.rs` | 3133 |
| `src/App.tsx` | 1429 |
| `src/locales.ts` | ~1583 |
| ClipboardItem 相关组件 | 体量大，职责过重 |

---

## 4. 资源占用

### 4.1 磁盘

| 项 | 量级 | 建议 |
|----|------|------|
| `src-tauri/target` | **~15GB**（debug 为主） | `cargo clean`；CI 缓存；勿提交 |
| `node_modules` | 可观 | 正常依赖，勿进 git |

### 4.2 运行时 CPU / 线程 / 锁

| 热点 | 风险 |
|------|------|
| 全局 `Arc<Mutex<Connection>>`（`DbState`） | 全库串行；未见 `busy_timeout`，争用易失败 |
| WAL + `auto_vacuum=FULL` | 写入时额外 IO |
| MQTT 每消息新线程 | 突发时线程/CPU 尖刺 |
| Cloud sync 1s sleep 轮询 | 常驻；cancel 不彻底时白烧 |
| 文件预览 2s 轮询 × N 线程 | 线程数随 entry 线性涨 |
| 加密队列 job 间 30ms sleep | 批量加解密变慢 |

### 4.3 内存 / 存储策略

**做得好：**

- Session history 上限 500
- 图片落文件
- UI 截断 content/html（`history_cmd`）
- FE `PAGE_SIZE=80`，搜索 limit 200

**仍有风险：**

- HTML 仍可能整段进 DB
- 有标签条目不参与 limit → 持久库无界
- 大富文本在同步路径上放大网络/内存

---

## 5. 做得好的地方

- 列表虚拟化（Virtuoso）+ 分页
- 历史内容/HTML 对 UI 截断
- 敏感标签触发加密的方向正确（但队列丢任务会削弱）
- Session 与持久化分流的设计意图清晰

---

## 6. 建议修复优先级

| 优先级 | 动作 |
|--------|------|
| **P0** | `contentType` 筛选参数 |
| **P0** | `enforce_limit` 是否包含 tagged 项（产品决策 + 实现） |
| **P0** | Windows 监听接 `windows_api` / 平台 cfg |
| **P0** | 生产关闭 `devtools` |
| **P1** | MQTT 真重启 + 有界 worker |
| **P1** | Cloud sync cancel/exit/JoinHandle |
| **P1** | paste_queue 失败续跑 |
| **P1** | 加密队列 poison/requeue |
| **P2** | 版本单一源；删未用 react-window / 明确 sql plugin 去留 |
| **P2** | 拆分 `cloud_sync.rs` / `clipboard/utils.rs` |
| **Ops** | 清理 `target`；文档标明双平台状态 |

---

## 7. 关键文件索引

| 主题 | 路径 |
|------|------|
| FE 类型筛选 | `src/shared/hooks/useHistoryFetch.ts` |
| 历史命令 | `src-tauri/src/app/commands/history_cmd.rs` |
| 存储上限 | `src-tauri/src/infrastructure/repository/clipboard_repo.rs` |
| MQTT | `src-tauri/src/services/mqtt_sub.rs` |
| Cloud Sync | `src-tauri/src/services/cloud_sync.rs` |
| 顺序粘贴 | `src-tauri/src/services/paste_queue.rs` |
| 加密队列 | `src-tauri/src/services/encryption_queue.rs` |
| 剪贴板监听 | `src-tauri/src/services/clipboard_listener.rs` |
| 预览 watch | `src-tauri/src/services/content_handler.rs` |
| DevTools | `src-tauri/tauri.conf.json` |
| 版本 | `versions.json`、`package.json`、`src-tauri/tauri.conf.json`、`src-tauri/Cargo.toml` |
| 虚拟列表 | `src/features/clipboard/components/VirtualClipboardList.tsx` |
| DB 初始化 | `src-tauri/src/database.rs` |

---

## 8. 结论

`tiez-clipboard` 功能面完整，但高置信度问题会直接伤体验或稳定性：

1. **类型筛选 IPC 参数命名** — 功能可能静默失效  
2. **存储上限对标签豁免** — 磁盘/DB 无界增长  
3. **Windows 监听半残** — 与双平台版本轨冲突  
4. **MQTT/Cloud 生命周期弱** — 配置变更/停止不可靠  
5. **MQTT 无界线程 + 全局 DB Mutex** — 资源尖刺与锁争用  
6. **生产 `devtools: true`** — 发布安全风险  

工程债：~15GB `target`、God 文件、重复依赖。不立刻炸功能，但拖垮维护效率。

---

## 9. 后续

若进入修复，建议顺序：

1. `contentType` 参数  
2. `enforce_limit` 策略  
3. MQTT/Cloud 生命周期  
4. 生产关 `devtools`  
5. Windows 监听修通  

本报告仅为 `tiez-clipboard` 审查交付，不包含代码修改。
