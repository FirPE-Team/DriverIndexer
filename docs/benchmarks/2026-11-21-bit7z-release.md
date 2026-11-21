# main 与 bit7z 的 release 性能对比

测试日期：2026-11-21。结论：**小包和短操作有明确收益，大包吞吐没有普遍提升。**
VMCI、Display 的整体索引耗时分别减少 17.6%、15.0%；System 整体索引基本持平，
其批量 INF/CAT 提取慢 13.1%，全量解压慢 4.7%。两种实现解压出的文件内容及生成的索引一致。

## 条件与方法

- 基线为本地 main `dc6996a999c201082acf635bb14f27e761a5ac42`；被测实现为 `21f568f` 的归档代码。
  相同的 benchmark harness 在 `d8ee818` 加入，并分别编译进两边；只读测试输出的清理修复为 `dd60b43`。
- Windows 10 Pro 10.0.19045 x64；i5-12400，6 核 12 线程，系统识别物理内存 31.74 GiB。
  Rust/Cargo 1.98.1，MSVC 14.38.33130，目标 `x86_64-pc-windows-msvc`。
- 两边使用相同 release 配置：`opt-level = 'z'`、LTO、单 codegen unit、`panic = 'abort'`，均未使用 UPX。
  两份 Cargo.lock 的包集合只相差新增的本地 bit7z crate；共同依赖版本一致。
- 两边使用同一份 7-Zip 26.00 DLL，SHA-256：
  `bbd705e3b58ca7677c1e9e67473f166a6712da034dcb567d571fbb67507a443f`。
  可执行文件旁不放置 `7z.exe` 或 `7z.dll`，测试默认内嵌资源部署方式。
- 热文件缓存，不清空系统缓存。每个普通场景先各预热 2 次，再各测 10 次；压缩先各预热 1 次，再各测 6 次。
  每轮交替使用 main→bit7z、bit7z→main 的顺序，串行执行，总计 **336 次正式测量**，未剔除慢样本。
- 主表为 Python `perf_counter_ns` 测得的进程整体墙钟中位数，包含启动、资源初始化和进程退出。
  测试输出的内容校验及清理在计时结束后执行。Rust harness 另外记录初始化、归档调用和内部总耗时。
  `operation_ms` 从 SevenZip 初始化完成计至归档方法返回，包含旧版调用子进程的时间。
- 解压预热结果与独立 7z CLI 参考结果逐文件比较路径、长度和 SHA-256；压缩预热结果通过独立 CLI 的 CRC 测试和解压内容比较。
  每次整体索引运行都解析 JSON 并进行完整相等比较。System 索引包含 375 个 INF、79,450 个硬件条目。
- System 的部分文件带只读属性。首轮在清理测试输出时中断；修正脚本后补测 System 剩余场景，保留已完成的全部样本。
  两阶段使用的被测二进制完全相同，合并后每组的样本数均满足上述约定。

## 输入

| 驱动包 | 压缩大小 / 字节 | 解压后 / 字节 | 文件数 | INF | CAT |
| --- | ---: | ---: | ---: | ---: | ---: |
| VMwareVMCI.7z | 56,287 | 118,867 | 4 | 1 | 1 |
| DisplayVmWare.7z | 12,820,036 | 60,644,245 | 35 | 1 | 1 |
| System.7z | 33,683,344 | 230,391,151 | 1,274 | 375 | 324 |

System 来自用户额外提供的文件；原始驱动包及解压内容均未提交到仓库。
指定目录测试分别选择 `VMware VMCI Bus Device`、`2019711`、
`CardReader/Realtek/51xx_10.0.22621.31277`，其中 System 目录包含 4 个文件。

## 整体索引：最直接的用户流程

运行真实 release CLI 的 `index` 命令，计入归档处理、INF 解析、CAT 验证、源指纹计算、索引保存和启动退出。
表中的百分比均为 `(main - bit7z) / main`，正值表示节省耗时。

| 驱动包 | main / ms | bit7z / ms | 耗时减少 |
| --- | ---: | ---: | ---: |
| VMwareVMCI | 69.433 | 57.244 | **17.56%** |
| DisplayVmWare | 78.887 | 67.063 | **14.99%** |
| System | 6,030.170 | 5,995.176 | **0.58%，基本持平** |

System 的 main P10–P90 为 5,964.547–6,111.170 ms，bit7z 为 5,905.830–6,199.612 ms。
配对耗时差的中位数 bootstrap 95% 区间跨过零（约 -24.8 至 +74.0 ms），无法据此认定其整体索引有稳定提升。

## 归档操作

以下也是进程整体墙钟中位数，包括每次创建 SevenZip 所需的资源准备。
“驱动包检测”调用 `is_driver_package`；“INF/CAT”在一次调用中合并两个筛选条件；
“目录压缩”使用同一份独立 CLI 解压出的目录，最高压缩等级、8 MiB 固实块、自动线程数。

| 驱动包 | 操作 | main / ms | bit7z / ms | 耗时减少 |
| --- | --- | ---: | ---: | ---: |
| VMwareVMCI | 驱动包检测 | 28.932 | 18.049 | +37.61% |
| VMwareVMCI | INF/CAT 提取 | 31.080 | 19.914 | +35.93% |
| VMwareVMCI | 指定目录提取 | 33.493 | 22.476 | +32.89% |
| VMwareVMCI | 全量解压 | 33.551 | 22.567 | +32.74% |
| VMwareVMCI | 目录压缩 | 42.178 | 31.236 | +25.94% |
| DisplayVmWare | 驱动包检测 | 29.040 | 18.174 | +37.42% |
| DisplayVmWare | INF/CAT 提取 | 30.700 | 19.826 | +35.42% |
| DisplayVmWare | 指定目录提取 | 516.373 | 507.754 | +1.67% |
| DisplayVmWare | 全量解压 | 517.508 | 509.567 | +1.53% |
| DisplayVmWare | 目录压缩 | 6,914.718 | 6,833.270 | +1.18% |
| System | 驱动包检测 | 30.723 | 19.628 | +36.11% |
| System | INF/CAT 提取 | 366.066 | 414.196 | **-13.15%** |
| System | 指定目录提取 | 993.408 | 972.163 | +2.14% |
| System | 全量解压 | 1,800.350 | 1,884.677 | **-4.68%** |
| System | 目录压缩 | 18,808.603 | 18,754.713 | +0.29% |

System 的 INF/CAT 回退较稳定：main P10–P90 为 357.579–388.050 ms，bit7z 为 401.980–442.099 ms；
配对耗时差 95% 区间约 -63.6 至 -35.5 ms。全量解压的对应区间约 -93.6 至 -59.3 ms，均未跨零。
Display 全量解压、Display 压缩和 System 压缩的配对区间跨零，宜视为近似持平。
bootstrap 使用每组 6 或 10 个配对样本、10,000 次重采样及固定种子；它只描述本次样本，不代表跨机器保证。

额外重压诊断（不计入 336 次正式样本）确认：两边的 7z 压缩方法和固实块数相同，
VMCI/Display/System 分别为 2/5/33 块，但压缩后字节数略有差异。因此压缩时间还应结合压缩率评估：

| 输入目录 | main 输出 / 字节 | bit7z 输出 / 字节 | 大小增加 |
| --- | ---: | ---: | ---: |
| VMwareVMCI | 55,930 | 56,050 | +0.21% |
| DisplayVmWare | 13,657,544 | 13,774,221 | +0.85% |
| System | 53,318,070 | 53,627,936 | +0.58% |

压缩结果均通过解压内容一致性检查，未要求压缩数据逐字节一致；本次没有看到大目录压缩速度或压缩率的明显提升。

## 解读

小包每次归档调用大致节省 11 ms。排除资源初始化及 harness 启动后，VMCI 的驱动包检测从 13.246 ms 降至 0.747 ms；
初始化则从 1.643 ms 增至 2.691 ms，仍被归档调用节省的时间覆盖。这与减少 `7z.exe` 子进程启动开销的解释一致。

大量文件时，收益会被额外处理抵消。从代码可确认，适配层筛选时枚举条目，crate 的 `extract` 内部又枚举一次，
随后针对选中路径逐段做 `symlink_metadata` 和越界检查。这些是优先剖析的候选开销；
**本次未对各内部步骤做 profiler 分解，不能断言全部回退都来自这些检查。**
整体索引还执行大量 INF/硬件信息处理、签名验证和指纹计算，这些路径在迁移中未修改。

release 主程序大小从 4,249,600 字节降至 3,851,776 字节，减少约 9.36%。
这一结果与运行耗时分开记录。

## 复现与数据

相同 harness 位于 `benches/archive_backend.rs`。分别在 main 源码副本和当前分支构建：

```powershell
cargo build --release --offline --bin DriverIndexer --bench archive_backend
```

main 的隔离源码副本通过 `git archive main` 创建，复制相同 `.env`、Cargo.lock 和 harness，
追加相同的 `[[bench]] name = "archive_backend", harness = false` 配置；
若副本放在本仓库的 target 下，还需追加独立 `[workspace] resolver = "3"`。
构建不改动 main 的生产源码。使用 Cargo 打印的 bench 可执行文件路径，
将两边的主程序和 harness 分别复制为以下目录中的 `DriverIndexer.exe`、`archive_backend.exe`。
两个目录中均不要放归档运行库，以保持内嵌资源测试条件。

```powershell
python scripts/benchmark_archives.py `
  --main-bin target/bit7z-benchmark/bin/main `
  --bit7z-bin target/bit7z-benchmark/bin/bit7z `
  --oracle target/bit7z-benchmark/main-source/assets-x64/7z.exe `
  --archive 'VMwareVMCI=testDrive/VMwareVMCI.7z' `
  --archive 'DisplayVmWare=testDrive/DisplayVmWare.7z' `
  --archive 'System=D:\UserData\Desktop\System.7z' `
  --output target/bit7z-benchmark/new-comparison `
  --repeats 10 --compression-repeats 6 --warmups 2
```

需要 Python 3.12+；输出目录必须为空。修复后的脚本可一次完成所有场景。
索引结果逐次核对；测试不安装驱动、不启动自解压 EXE。

- [原始 336 次样本 CSV](bit7z-release-2026-11-21/samples.csv)
- [统计结果、P10/P90 和配对区间](bit7z-release-2026-11-21/summary.json)
- [硬件、版本、二进制哈希和输入摘要](bit7z-release-2026-11-21/environment.json)
- [压缩方法、固实块数和输出大小诊断](bit7z-release-2026-11-21/compression.json)

测试覆盖本机 x64 release、热缓存和默认内嵌部署；未覆盖冷启动、其他 CPU/架构、XP/Win7、实际安装驱动或并发安装吞吐。
