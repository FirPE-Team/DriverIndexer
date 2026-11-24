# 驱动管理对照验收记录

验收日期：2026-11-19。开发分支：`codex/dismpp-driver-management`。
静态依据及函数地址见 [Dism++ 基准行为记录](dismpp-driver-management.md)。

## 环境与边界

| 项目 | 本轮环境 |
| --- | --- |
| 运行宿主 | Windows 10 x64，build 19045 |
| 可丢弃离线目标 | `D:\Project\FirPE\Mount`，Windows 11 x64，build 22621 |
| Rust | `rustc 1.98.1 (48a229cea 2026-09-01)` |
| 编译目标 | `x86_64-pc-windows-msvc`、`i686-pc-windows-msvc`、`aarch64-pc-windows-msvc` |
| 发布构建 | VC-LTL5 / YY-Thunks，`target/release/DriverIndexer.exe` |
| 基准 GUI | Dism++x64 10.1.1002.1，用户提供的 10.1.1002 系列资料对应二进制 |

本轮真实导入、删除全部限定在上述离线目标。在线导入、绑定设备处理及删除通过模拟后端检查操作顺序，
没有在当前系统执行。默认测试不修改真实驱动库。离线闭环测试被标记为 `ignored`，要求显式环境变量，
并校验目标的规范路径及离线状态。

GUI 操作使用用户授权的本机 lcr，目标栏选择 `D:\Project\FirPE\Mount`。
从 refs 启动的基准转到安装目录 `C:\Program Files\WindsysTool\Dism++`；两处文件 SHA-256 相同：

| 文件 | SHA-256 |
| --- | --- |
| `Dism++x64.exe` | `16bbdb339173d25b4332b377da96e80809aabfe6739cf35d5e70484f08cfdc42` |
| `Config/amd64/CBSHost.dll` | `6a1075f4fab4acd2722e8946cba7b23508d120367c84aae3e83284124227013a` |

## 双向对照结果

初始第三方发布包数量为 0。Display 覆盖普通包，VMCI 覆盖关键类别包。

| 元数据 | Display | VMCI |
| --- | --- | --- |
| 本轮单包导入后的发布名 | `oem0.inf` | `oem0.inf` |
| 原始 INF 名 | `vm3d.inf` | `oem2.inf` |
| 厂商 | VMware, Inc. | VMware, Inc. |
| 类别 | Display / 显示适配器 | System / 系统设备 |
| 类别 GUID | `4d36e968-e325-11ce-bfc1-08002be10318` | `4d36e97d-e325-11ce-bfc1-08002be10318` |
| 版本 | 8.16.7.0 | 9.8.6.0 |
| 日期 | 2019-07-11 | 2016-05-25 |
| 关键标识 | false | true |
| 仓库目录 | `vm3d.inf_amd64_eb377e04601865d7` | `oem2.inf_amd64_64189b30c198ca23` |
| 导出文件数 | 34 | 3 |
| 文件内容字节数 | 60,642,197 | 118,598 |
| 相对清单、各文件大小与 SHA-256 | 两边全部一致 | 两边全部一致 |

Display 的原压缩包有 35 个文件；入库后的包不包含 `vm3dver.dll`，两边导出均为 34 个文件。
比较以实际仓库内容为准。VMCI 的磁盘分配大小为 122,880 字节，基准界面显示 120 KB。
最终 CLI 的 `Size` 使用分配大小，`File Bytes` 使用逻辑大小；早期证据文本在该修正之前生成，
其中 `Size` 是逻辑大小，不应据此解释最终输出。

执行顺序及结果：

1. 基准导入 Display，本项目枚举，元数据与基准一致。
2. 双方分别导出 Display，逐文件对比通过。
3. 本项目重复导入，成功且发布包数量不增加；本项目按发布名删除成功。
4. 本项目重新导入 Display，基准识别并删除成功。
5. 本项目导入 VMCI，基准识别关键类别，双方导出逐文件对比通过。
6. 本项目删除 VMCI 成功；重新导入后，基准删除成功。
7. x86-only 合成 INF 在 x64 镜像上，双方均为成功 0、不适用 1，未入库。
8. 显式启用 CLI 自动闭环测试，Display 和 VMCI 分别通过：导入、重复导入、枚举、
   按发布名及原始名导出、逐文件内容比较、删除与最终包集合复核。

## 补充 CLI 与错误路径验收

| 场景 | 结果 |
| --- | --- |
| 两个不同目录下同名 `driver.7z`，通配符一次导入 | 成功 2、失败 0、不适用 0；临时目录不同；新增 `oem0.inf`、`oem1.inf`，均已清理 |
| 损坏 INF 后跟不适用 INF | 两项均被处理；成功 0、失败 1、不适用 1；损坏项保留 `0xE0000100`，不适用项为 `0x8007000A`；退出非零 |
| 损坏、适用 VMCI、不适用 INF 混合 | 成功 1、失败 1、不适用 1；退出非零；新增 VMCI 发布包已清理 |
| Unicode 厂商及原始 INF 名 | 默认逻辑测试覆盖 `ÉDITEUR` / `éditeur`、`Ädapter.inf` / `ädapter.INF` 的大小写匹配 |
| 空第三方列举 | 退出 0 |
| 导出无匹配 INF | 输出错误，退出 1 |
| 删除无匹配 INF | 输出错误，退出 1 |
| 删除没有选择条件或 `--all` | 参数错误，退出 2 |
| `--system` 与 `--system-only` 同时指定 | 参数冲突，退出 2 |
| 非 OEM 包删除 | 模拟后端验证明确失败，未进入设备处理或仓库删除 |
| 主操作失败及辅助反射失败 | 模拟后端验证批量继续；主操作失败影响退出码，辅助失败保留成功结果并附消息 |
| 仓库／包句柄释放 | 假 ABI 测试确认元数据错误提前返回时，各自关闭一次 |

最后一轮自动闭环完成后，再次读取 `Windows/INF/oem*.inf`，数量为 **0**，与初始状态一致。
仅清理本轮新增发布名，没有删除其他镜像内容。最后补拍空列表截图时 lcr 桌面捕获返回
`BitBlt: invalid handle (6)`，因此最终状态依据文件枚举和闭环检查；已有 GUI 操作截图仍保留。

## 工程检查

以下命令均通过，使用本地依赖缓存执行：

```powershell
cargo fmt -- --check
cargo check --all-targets --offline
cargo test --all-targets --offline
cargo clippy --all-targets --all-features --offline -- -D warnings
cargo check --all-targets --target i686-pc-windows-msvc --offline
cargo check --all-targets --target aarch64-pc-windows-msvc --offline
cargo build --release --offline
```

默认测试：lib 26 项、binary 27 项、INF integration 2 项通过；离线写入 1 项和既有 Windows 手工测试
2 项默认忽略。lib 与 binary 包含共同模块的重复测试，以上数字是测试运行次数，不是独立场景数。
离线闭环另显式运行两次，两种包均通过。复现入口见 README 的“诊断与测试”。

## 证据位置与未覆盖组合

本机证据在被忽略的 `target/driver-validation/`，不随 Git 提交，不包含在发行包中：

- `baseline-empty.png`、`baseline-import-display.png`、`baseline-display-details.png`、
  `baseline-vmci-details.png`：基准目标与元数据。
- `baseline-export-display/` 与 `rust-export-display/`、`display-manifest.json`：Display 内容对照。
- `baseline-export-vmci/` 与 `rust-export-vmci/`、`vmci-manifest.json`：VMCI 内容对照。
- `duplicate-import.txt`、`rust-remove-display.txt`、`rust-remove-vmci.txt`：本项目操作结果。
- `baseline-remove-rust-display.png`、`baseline-remove-rust-vmci.png`：基准删除本项目导入包。
- `baseline-not-applicable.png`、`rust-not-applicable.txt`：不适用分类对照。
- `same-name-import.txt`、`same-name-list.txt`、`mixed-import.txt`、`mixed-success-import.txt`、`cli-*.txt`：补充回归。

开发中的 `probe_reference.py` 是只读 CBSHost 工作函数试验，不代替 GUI 对照证据。
Ghidra 伪代码和补充反汇编属于静态证据；上面的双向操作属于真实运行观察。

| 组合 | 覆盖程度 |
| --- | --- |
| x64 Win10 宿主服务 x64 Win11 离线镜像 | 四项功能与普通／关键包真实闭环通过 |
| 当前系统在线导入／删除 | 模拟后端；没有真实写入 |
| x86、ARM64 | 所有目标编译检查通过；未实测运行 |
| Win7、其他旧系统 | 保留选择 DLL、版本布局和兼容分支；未实测运行 |
| 其他宿主／镜像架构组合 | 未实测运行 |

本项目默认不依赖、不加载、不分发 Dism++ 或 CBSHost.dll；它们仅用于研究与对照验收。
