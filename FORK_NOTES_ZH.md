# CC Switch Codex Proxy 兼容版

此分支在 CC Switch 的本地代理中加入了 Codex Responses 请求清洗功能，用于解决部分第三方中转站返回以下错误的问题：

```text
403 Forbidden: Image generation is not enabled for this group
```

## 行为

当以下条件全部满足时，代理会递归删除请求中与 `image_generation` 相关的对象：

- 当前供应商已开启“生图兼容过滤”；
- 应用类型是 Codex；
- 请求路径是 `/responses` 或 `/v1/responses`；
- 上游使用原生 Responses 协议；
- 当前供应商不是 OpenAI 官方供应商。

删除规则与 `BINGWU2003/codex-proxy` 的行为一致：

- `type` 是 `image_generation`、`image_generation_call` 或 `image_generation_preview`；
- `name` 不区分大小写且包含 `image_generation`；
- 当前 Codex Desktop 使用的 `image_gen` namespace 或 `imagegen` 函数。

其他工具（例如 `apply_patch`）、图片输入和官方 OpenAI 请求保持不变。

开关位置：编辑第三方 Codex 供应商 → 高级选项 → 生图兼容过滤。该开关仅在“Responses（原生）”格式下显示。

## 构建 macOS 安装包

```bash
npm install
npm run typecheck
npm run test:unit
cargo test --manifest-path src-tauri/Cargo.toml
./node_modules/.bin/tauri build --config src-tauri/tauri.codex-proxy.conf.json --bundles app,dmg
```

兼容版使用独立应用标识 `com.ccswitch.codexproxy`，产品名为 `CC Switch Codex Proxy`，并禁用官方更新端点，避免更新为不含本功能的官方二进制。

## 同步上游

约定 `origin` 指向个人 fork，`upstream` 指向官方仓库：

```bash
git remote add upstream https://github.com/farion1231/cc-switch.git
git fetch upstream
git switch main
git merge --ff-only upstream/main
git push origin main
git switch feat/codex-image-generation-sanitizer
git rebase main
git push --force-with-lease origin feat/codex-image-generation-sanitizer
```

发生冲突时重点检查 `src-tauri/src/proxy/forwarder.rs` 中调用 `strip_image_generation_items` 的位置，然后重新运行完整测试和构建命令。
