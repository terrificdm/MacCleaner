# MacCleaner

[English](README.md)

一款运行在 Apple 芯片 Mac 上的磁盘清理和应用卸载工具。完全在本机运行，不联网，不收集任何数据。使用 Tauri 2（Rust）和 React 开发。

## 功能

| 模块 | 作用 |
|---|---|
| 智能扫描 | 依次运行系统垃圾、开发者垃圾、残留文件、下载、大文件和旧文件、废纸篓（废纸篓需要完全磁盘访问权限），汇总可以清理的空间。“清理”按钮只处理前四项；大文件和废纸篓需要你自己查看。 |
| 空间透视 | 扫描整个内置数据卷，用可以逐层点进去的旭日图显示空间占用。允许删除的项目可以直接移到废纸篓。 |
| 系统垃圾 | 应用缓存（`~/Library/Caches`）、日志、崩溃报告，以及 3 天没有改动的临时文件。 |
| 开发者垃圾 | npm / pip / Yarn / pnpm / Bun / Cargo / Go / Gradle / Maven / Xcode 缓存，以及 90 天没改动的项目里的 `node_modules`。另有两个可选的命令项，默认不勾选：`brew cleanup --prune=all`（Homebrew 缓存和旧版本）和 `docker system prune -f`（已停止的容器和构建缓存）。 |
| 大文件和旧文件 | 个人文件夹里的大文件和很久没用的文件。默认标准：500 MB 以上，或者 100 MB 以上且一年没用过；标准可以调整，也可以按类型筛选。默认全都不勾选。 |
| 应用卸载 | 列出 `/Applications` 和 `~/Applications` 里的应用，显示大小、来源（App Store / Homebrew / 手动安装）和上次使用时间，连同关联文件一起卸载。 |
| 残留文件 | 已经删除的应用留下的配置、缓存和数据。 |
| 废纸篓与下载 | 清空废纸篓；整理 `~/Downloads`（放了一个月以上的安装包会被建议清理）。 |
| 清理历史 | 每次清理和演练的记录，只保存在本机。 |

界面支持简体中文和英文，外观可选浅色、深色或跟随系统。

## 如何保证文件安全

- **默认开启演练模式。** 清理时只列出将要处理的内容。确认结果可靠后再到设置里关闭。这一规则在 Rust 后端强制执行，不只是界面上的开关。
- **文件和文件夹都移到废纸篓**，可以放回原处。属于管理员的项目由“访达”移动，系统会要求输入密码。只有两类操作会直接永久删除，确认前都会单独提醒：清空废纸篓，以及 `brew cleanup` / `docker system prune` 命令项。
- **卸载 Homebrew 安装的应用时**，应用会像其他应用一样先移到废纸篓，然后运行 `brew uninstall --cask`，让 Homebrew 删除它的记录。如果这个 cask 定义了卸载步骤，Homebrew 也会执行。
- **每个文件和文件夹在真正移动前都会重新检查**（`src-tauri/src/safety.rs`）：
  - 永远不删：macOS 系统目录、钥匙串、隐私权限数据库、Apple 应用的数据、照片和音乐资料库、MacCleaner 自己；
  - 不单独删除应用包、iWork 文稿、磁盘映像包、Git 仓库 `.git` 目录里面的文件（只能整个删）；
  - 不删 `~/Library/Caches`、`~/Downloads`、`~/Projects` 这类顶层文件夹本身，只清理其中的内容；
  - 路径在白名单里、扫描后大小有变化、是正在运行的应用的缓存、或者正被任何进程打开时，都会跳过。
- **白名单。** 默认包含个人数据位置（文稿、`.ssh`、`.gnupg`、iCloud 云盘、云存储、邮件、信息、Safari、账户、日历、通讯录、通话记录、iPhone 备份、照片）。白名单里的路径永远不会被移动；命令项和清空废纸篓不按路径操作，不受白名单约束。这些条目在设置里标有“默认”，可以移除或恢复；也可以添加自己的路径。匹配时不区分大小写。
- 扫描结果只保存在内存里；设置和历史保存在 `~/Library/Application Support/MacCleaner/`。

## 系统要求

- Apple 芯片（M 系列）的 Mac，不支持 Intel Mac。
- macOS 15 或更新版本。在 macOS 26 上测试过；macOS 15 和 27 理论上可用，但尚未测试。
- MacCleaner 使用自签名证书签名，没有经过 Apple 公证（见下方“首次打开”）。

## 安装

### 方式 A：通过 MacCleaner 的 `.dmg`

1. 打开 dmg，可以把 MacCleaner 拖到“应用程序”文件夹；也可以直接双击 dmg 里的 MacCleaner，选择“安装并打开”（见[更新](#更新)）。dmg 里也附带了中英文两版 README。
2. 第一次打开会被系统拦下，因为它没有经过公证。双击 MacCleaner，点“完成”；然后打开“系统设置 → 隐私与安全性”，滚到最下面的“安全性”部分，点 MacCleaner 旁边的“仍要打开”，输入密码。MacCleaner 随后会直接打开（如果没有打开，再打开一次并点“打开”）。
   熟悉终端的话，也可以运行 `xattr -dr com.apple.quarantine /Applications/MacCleaner.app`，效果相同。
   公司管理的 Mac 可能完全不允许这样打开。

### 方式 B：从源码构建

需要先安装：Command Line Tools（`xcode-select --install`）、通过 [rustup](https://rustup.rs) 安装的 Rust、Node.js 20.19 以上或 22.12 以上。

```bash
npm install
./scripts/setup-signing.sh   # 每台 Mac 只需一次：创建本地签名身份
./scripts/build-install.sh   # 构建、签名并安装到 /Applications/MacCleaner.app
```

`setup-signing.sh` 会在一个单独的钥匙串（`~/Library/Keychains/maccleaner-signing.keychain-db`）里创建自签名证书，钥匙串密码保存在你的登录钥匙串里。签名身份保持不变，你授予的权限在重新构建后依然有效。

请始终运行 `/Applications` 里的那一份。`src-tauri/target/` 里的构建输出每次构建都会被覆盖。

### 方式 C：直接拷贝构建好的 `MacCleaner.app`

如果别人直接给你应用本身（比如压缩成 zip），直接在它所在的位置打开，选择“安装并打开”；也可以自己把它移到 `/Applications`。首次打开时需要按方式 A 的第 2 步操作。

## 首次打开和日常使用

1. 欢迎页会介绍安全机制。授予“完全磁盘访问权限”：点“打开系统设置”→ 隐私与安全性 → 完全磁盘访问权限 → 打开 MacCleaner 的开关（没有的话点“+”添加），然后退出并重新打开 MacCleaner。没有这个权限，就读不到废纸篓、其他应用的沙盒容器和部分系统目录。
2. 从“智能扫描”开始，或者在侧边栏打开某个模块，点“扫描”。
3. 查看列表：取消勾选要保留的项目；点眼睛图标可以快速查看；右键可以把路径加入白名单。
4. 点“清理”，查看确认清单后确认。演练模式下不会移动任何文件，结果页会显示实际执行时会发生什么。
5. 确认结果没问题后，到“设置”里关闭演练模式。之后清理的项目会进入废纸篓，清空废纸篓后才会真正释放空间（废纸篓与下载 → 清空废纸篓）。

MacCleaner 第一次移动属于管理员的项目或清空废纸篓时，系统会询问是否允许它控制“访达”，请选择允许。

## 更新

直接在新版本所在的位置打开它（dmg、下载文件夹、zip 解压出的位置都可以）。MacCleaner 从“应用程序”以外的位置运行时，会询问是否安装：

- 还没有安装：“安装并打开”；
- 已安装旧版本：显示两个版本号，提供“更新并重新打开”；
- 已安装相同或更新的版本：提供重新安装或替换，替换成较旧版本时会有提醒；
- “应用程序”里有另一个同名的 MacCleaner.app：拒绝安装，请你先处理它。

确认后，它会请正在运行的旧版本退出，把已安装的版本移到废纸篓（需要时可以放回），把自己复制到 `/Applications`，再从那里重新打开。如果你的账户没有写入 `/Applications` 的权限，系统会要求输入管理员密码。设置、白名单和历史都会保留。

也可以先退出 MacCleaner，再把新版本拖到“应用程序”并在访达里选择“替换”。开发者可以运行 `./scripts/build-install.sh`，它会先退出正在运行的旧版本。

只要新版本用同一个签名身份签名（在同一台 Mac、用同一个签名钥匙串构建），完全磁盘访问权限就会保留。新下载的版本第一次打开时，可能需要再点一次“仍要打开”。

## 卸载

1. 退出 MacCleaner。
2. 把 `/Applications/MacCleaner.app` 移到废纸篓。
3. 可选：如果要清除它创建的所有内容，把下面这些也移到废纸篓（有些可能不存在）：
   - `~/Library/Application Support/MacCleaner`（设置、白名单、历史）
   - `~/Library/Caches/app.maccleaner.MacCleaner`
   - `~/Library/WebKit/app.maccleaner.MacCleaner`
   - `~/Library/HTTPStorages/app.maccleaner.MacCleaner`
   - `~/Library/Saved Application State/app.maccleaner.MacCleaner.savedState`
4. 可选：清除它的隐私权限（完全磁盘访问、控制访达）：
   ```bash
   tccutil reset All app.maccleaner.MacCleaner
   ```
5. 只有在你自己构建过、并且以后不再构建时，才需要删除签名身份：
   ```bash
   security delete-keychain ~/Library/Keychains/maccleaner-signing.keychain-db
   security delete-generic-password -s maccleaner-signing-keychain
   ```

## 打包发布

所有步骤都在你自己的 Mac 上完成，不会上传任何东西。

1. 每台 Mac 只需一次：按方式 B 安装所需工具，运行 `npm install`，再运行 `./scripts/setup-signing.sh`。
2. 在 `src-tauri/tauri.conf.json` 里修改版本号（`version`）。`package.json` 和 `src-tauri/Cargo.toml` 里的版本号也改成同一个。
3. 运行检查：`npm run typecheck && npm run check:i18n && npm test && (cd src-tauri && cargo test)`。
4. 生成 dmg：
   ```bash
   ./scripts/package-dmg.sh              # 构建、签名并打包
   ./scripts/package-dmg.sh --no-build   # 只把上一次的构建重新打包
   ```
   输出：`release/MacCleaner_<版本>_aarch64.dmg`（不纳入 git）及其 SHA-256 校验值。dmg 里包含 `MacCleaner.app`、指向“应用程序”的快捷方式，以及中英文两版 README。
5. 把 dmg 和校验值发给对方，对方按[安装](#安装)一节操作。

每次发布都在同一台 Mac、用同一个签名钥匙串构建，这样更新后大家已经授予的权限不会失效。如果钥匙串丢了，就会生成新的签名身份，所有人都要重新授予完全磁盘访问权限。为了保险，建议把 `~/Library/Keychains/maccleaner-signing.keychain-db` 连同它的密码（`security find-generic-password -s maccleaner-signing-keychain -w`）一起备份，比如存进密码管理器。

## 开发

```bash
npm run dev          # 在浏览器里用假数据预览界面：http://localhost:1420
                     # 例如 ?theme=light&lang=en&page=space、?onboarding、?nofda
npm run tauri dev    # 运行真实应用（权限跟随终端）
npm run typecheck
npm test             # 前端单元测试（vitest）
npm run check:i18n   # 检查中英文 key 一致、用到的 key 都存在
npm run icons        # 从 src/assets/logo.svg 重新生成图标

cd src-tauri
cargo test                                         # 单元测试和接口测试（见下方说明）
cargo test real_probe  -- --ignored --nocapture    # 只读扫描本机
cargo test large_probe -- --ignored --nocapture    # 只读扫描大文件
cargo test smart_probe -- --ignored --nocapture    # 只读：智能扫描会默认勾选哪些
cargo test disk_probe  -- --ignored --nocapture    # 只读扫描整个磁盘
```

`cargo test` 只操作临时目录，并使用单独的设置目录。其中一个测试会把它自己创建的临时文件移到你的废纸篓，然后再从废纸篓里删掉。

如何生成 dmg，见[打包发布](#打包发布)。

### 项目结构

```
src/                 React 界面（pages/、components/、lib/api.ts、lib/mock.ts、i18n/）
src-tauri/src/
  lib.rs             Tauri 命令、菜单
  installer.rs       安装和更新到 /Applications
  safety.rs          判断什么可以移动（每个文件和文件夹都要经过它）
  cleaner.rs         移到废纸篓、演练模式、命令项、卸载
  disk.rs            空间透视扫描
  junk.rs            系统垃圾、开发者垃圾、大文件、废纸篓、下载
  apps.rs            已安装应用、关联文件、残留文件
  inuse.rs           正在被进程打开的文件（lsof）
  mac.rs             macOS 接口：废纸篓、运行中的应用、图标、访达、Spotlight
  settings.rs、history.rs
scripts/             签名、构建安装、dmg、图标、文案检查
```

## 已知限制

- 没有经过公证，其他 Mac 需要在“隐私与安全性”里放行一次。
- 卸载应用后，它的后台登录项要到下次重启才会停止运行。
- Homebrew cask 自带的卸载步骤由 Homebrew 执行，不经过 MacCleaner 的检查。
- APFS 克隆文件能释放多少空间只是估算。
- 不扫描外接磁盘和网络磁盘。
- macOS 上无法自动测试真实的应用窗口；通过访达、`brew`、`docker` 删除以及退出应用这些流程，是手动验证的。

## 许可证

[MIT](LICENSE)。MacCleaner 会移动和删除你 Mac 上的文件，本软件按“原样”提供，不附带任何担保。在确认结果可靠之前，请保持演练模式开启。
