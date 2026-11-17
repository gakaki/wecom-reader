# WeCom Reader

企业微信桌面风格的多平台 RSS 阅读器，用 [NSH](https://nsh.dev) + Rust 构建，GPUI Kit 模式渲染。数据源为 V2EX 与 Linux.do，只做列表和详情（无评论），每 5 分钟自动刷新。

线上站点：<https://wecom.gakaki.com/>（另可用 <https://wecom-reader.pages.dev/>）

## 特性

- 双数据源：V2EX（`blog.v2ex.com/rss.xml`）、Linux.do（`linux.do/latest.rss`，官方源失败时回退到社区镜像）
- 桌面三栏布局 / 移动端（Android、iOS）底部 TabBar + 大触摸目标 + 安全区适配
- Android 物理返回键：详情页返回列表
- Web 端同源 Pages Function 代理 RSS，规避跨域；每 5 分钟轮询刷新

## 目录结构

- `src/` — Rust 应用（`lib.rs` UI 与刷新逻辑、`rss.rs` 轻量 RSS/Atom 解析器、`main.rs`、`bin/gpui.rs` GPUI 入口）
- `functions/api/rss.js` — Cloudflare Pages Function，RSS 代理与源回退
- `worker/index.js`、`dev-server.mjs` — 本地开发用的代理
- `scripts/` — 构建辅助脚本（WASM/Windows 的 Zig 工具链包装、下载面板注入）
- `dist-web/` — Web 构建产物（已 gitignore，用下面命令重建）
- `nsh.json`、`wrangler.toml` — NSH 与 Cloudflare Pages 配置

## 构建

需要 Rust stable、[NSH CLI](https://nsh.dev)、Zig（WASM/Windows 交叉编译用）、JDK + Android SDK/NDK（Android 用）。

```bash
# 各平台（在 macOS 上）
nsh package . --target macos   --out .nsh/out/macos
nsh package . --target ios     --out .nsh/out/ios       # 真机未签名，需 --sign/--profile 签名
nsh package . --target ios-simulator --out .nsh/out/ios-simulator
IPHONEOS_DEPLOYMENT_TARGET=15.0 nsh package . --target ios --out .nsh/out/ios
nsh package . --target android --out .nsh/out/android   # 需 ANDROID_HOME / ANDROID_NDK_HOME / JAVA_HOME

# Windows（macOS 交叉编译，native host + llvm-lib shim）
PATH="$PWD/scripts:$PATH" AR_x86_64_pc_windows_msvc="$PWD/scripts/llvm-lib" \
  nsh package . --target windows --out .nsh/out/windows

# Web（WASM，Zig 作 clang 包装）
AR_wasm32_unknown_unknown="$PWD/scripts/zig-wasm-ar" \
CC_wasm32_unknown_unknown="$PWD/scripts/zig-wasm-cc" \
  nsh package . --target web --out .nsh/out/web
```

## Web 部署（Cloudflare Pages）

```bash
# 1) 同步 Web 产物并注入首页下载面板
rm -rf dist-web && mkdir -p dist-web
cp -r ".nsh/out/web/WeCom Reader/." dist-web/
python3 scripts/inject-download.py dist-web/index.html

# 2) 部署（Pages 项目 wecom-reader，绑定域名 wecom.gakaki.com）
npx wrangler pages deploy dist-web --project-name wecom-reader --branch main
```

RSS 代理由 `functions/api/rss.js` 提供，无需单独部署 Worker。

## 客户端下载

- Android：`https://pub-b0f6a4366b7e4d00a76a30bb008e9416.r2.dev/WeCom-Reader.apk`
- macOS：`https://pub-b0f6a4366b7e4d00a76a30bb008e9416.r2.dev/WeCom-Reader-macOS.zip`
- Windows：`https://pub-b0f6a4366b7e4d00a76a30bb008e9416.r2.dev/WeCom-Reader-Windows.zip`

> 当前 iOS 包未签名；Android 为 debug 签名。正式发布需替换签名材料。
