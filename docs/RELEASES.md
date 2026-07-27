# TieZ 客户端发布流程

`versions.json` 是客户端版本号的唯一来源，分别记录：

- `stable.windows` / `stable.macos`：正式版
- `beta.windows` / `beta.macos`：群测试版

构建脚本会把目标版本同步到 `package.json`、`src-tauri/Cargo.toml` 和
`src-tauri/tauri.conf.json`，并校验更新接口、发布渠道与 Git 标签。

## 群测试版

测试版必须使用 SemVer 预发布后缀，例如：

```json
{
  "beta": {
    "windows": "1.0.0-beta.1",
    "macos": "1.0.0-beta.1"
  }
}
```

在对应系统上构建：

```bash
npm ci
npm run release:beta:win
# 或
npm run release:beta:mac
```

构建结束后脚本会恢复 package、Cargo 和 Tauri 配置，只保留
`versions.json` 中由维护者明确修改的版本号，避免误提交 beta 生成状态。

构建前需要在本机环境中配置 Tauri 签名私钥。签名私钥不能提交到仓库、
上传到版本后台或发送到测试群。

测试版安装包会固定使用 `beta` 更新通道。服务端在 `channel=beta` 时会同时
比较 beta 与 stable 上已发布的更高版本，因此测试用户可以从
`1.0.0-beta.x` 通过「检查更新」升到正式版 `1.0.0` 及之后的 stable 补丁。
stable 用户仍只跟随 stable 渠道，不会收到仅发布在 beta 的构建。

构建完成后：

1. 将安装包发送给测试群。
2. 如需让测试用户自动更新，将 updater 制品和 `.sig` 上传到 HTTPS 地址。
3. 在 TieZ Analytics 后台创建 `beta` 发布批次，填写对应架构的 URL 和签名。
4. 测试通过后，把不带预发布后缀的版本写入 `stable`，再走正式发布。

如果只想单次发包、不提供后续自动更新，可以不在后台发布 beta 批次。

## 正式版

更新 `stable` 版本后，在对应系统构建：

```bash
npm run release:stable:win
# 或
npm run release:stable:mac
```

GitHub Actions 正式发布标签：

- Windows：`v1.0.0`
- macOS：`mac-v1.0.0`

工作流会阻止标签、平台版本、Cargo、Tauri 配置或更新通道不一致的构建。

## 便携版

Windows 便携版不会运行安装器自动更新。用户需要从官网或测试群下载新的
便携包并替换程序文件，原有 `data` 目录会继续保留。
