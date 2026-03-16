# DriverIndexer

简体中文 | [English](README.md)

## 简介

`DriverIndexer` 是一个高效、智能的驱动程序管理和安装命令行工具。它专注于解决传统驱动包安装速度慢、资源浪费的问题。

- ⚡ **按需解压** - 仅解压当前设备所需的驱动程序，大幅缩短安装时间。
- 🎯 **智能匹配** - 自动识别硬件 ID，精准匹配最佳驱动程序。
- 📦 **多格式支持** - 支持驱动程序目录、7z 压缩包等多种驱动包格式。
- ⚙️ **高性能** - 利用多线程和智能索引技术提升安装和管理速度。
- 🔄 **离线支持** - 支持对离线 Windows 系统进行驱动程序的导入、安装与管理，不依赖 DISM 环境。
- 🖥️ **命令行界面** - 支持静默安装和自动化部署，方便集成到维护脚本中。
- 🛠️ **驱动管理** - 提供系统驱动程序的导入、导出和删除功能。
- 🔧 **驱动打包** - 可将驱动与程序合并为单一自解压可执行文件（EXE）。

### `DriverIndexer`的价值？

传统的驱动包安装方式，需要将整个驱动包（可能包含几 GB 数据）全部解压，再调用 `Dpinst` 或类似工具进行安装，这不仅消耗大量时间，还浪费磁盘空间。

`DriverIndexer` 通过建立索引文件，实现了根据设备需求按需解压和自动安装的功能，极大地提升了安装效率。

### 索引文件是什么？

索引文件是 `DriverIndexer` 的核心。由于硬件 ID 列表存储在 INF 文件内部，我们首先需要创建 `硬件 ID 列表` 与 `驱动包内驱动文件路径`
的对应关系。这一关系即是索引（JSON 格式）。

通过索引，程序能够迅速确定设备所需的驱动程序，从而实现精准的按需解压和安装。

### 驱动包获取网站

> 我们更提倡自己下载、搜集驱动包，如有需求也可自行提取目前各个驱动软件内的驱动包（一般此类驱动包无版权）

以下为推荐的驱动包下载网站（均免费、无加密）

- [SamDrivers](https://driveroff.net)
- [DriverPack](https://drp.su/en/foradmin?_blank)
- [3DP](https://www.3dpchip.com/3dpchip/3dp/net_down.php?_blank)
- [DriverOff](https://driveroff.net/category/dp?_blank)
- [BatPEDriver](http://forum.ru-board.com/topic.cgi?forum=62&topic=24098&start=71&limit=1&m=1#1?_blank)

## 软件架构

使用`Rust`语言编写，调用 WindowsAPI 获取硬件信息、安装设备驱动。

### 驱动匹配规则

> 三次匹配（防止部分驱动未安装成功）

1. 匹配当前系统架构
2. 匹配当前操作系统版本
3. 匹配硬件信息

    - 设备硬件ID vs 驱动文件硬件ID
    - 设备硬件ID vs 驱动文件兼容ID
    - 设备兼容ID vs 驱动文件硬件ID
    - 设备兼容ID vs 驱动文件兼容ID

### 驱动排序规则

1. 签名状态（微软签名 > 其他签名 > 未签名）
2. 匹配分数（最强优先）
3. 驱动日期（最新优先）
4. 驱动版本（最新优先）

## 使用说明

本程序为命令行程序，故需要在其后面接参数运行，如直接双击程序将会出现“闪退”现象，您可通过`cmd`、`PowerShell`等终端来运行。  
注意：请使用**管理员身份**运行终端。

### 创建驱动索引文件

索引文件通常在首次使用驱动包时创建，后续如驱动包内容发生变动，需要重建索引。

`DriverIndexer.exe index <驱动包/目录路径> <索引文件保存路径>`

- 选项

| **参数**            | **短参数** | **描述**           |
|-------------------|---------|------------------|
| `--password <密码>` | `-p`    | 指定驱动包密码，用于解压驱动包。 |
| `-- compress`     | `-c`    | 压缩索引文件`。         |

- 示例
    - `DriverIndexer.exe index D:\netcard D:\index.json`
    - `DriverIndexer.exe index D:\netcard.7z D:\index.json`

### 安装驱动程序

使用索引文件或直接指定驱动包路径进行安装。

`DriverIndexer.exe install <驱动包/目录路径> [选项]`

- 驱动路径格式：压缩包（限 7zip 所支持的格式）、目录格式。
- 支持通配符(`*`、`?`)，用于匹配多个驱动包。
- 不使用索引时将自动创建临时索引

- 选项

  | **参数**                 | **短参数** | **描述**                              |
                                      |------------------------|---------|-------------------------------------|
  | `--index-path <路径>`    | `-i`    | 指定索引文件路径，用于加速安装。不指定时将自动创建临时索引。      |
  | `--password <密码>`      | `-p`    | 指定驱动包密码，用于解压驱动包。                    |
  | `--class <类别>`         | `-c`    | **包含**指定的驱动类别，仅安装匹配类别的驱动。多个类别可重复指定。 |
  | `--exclude-class <类别>` | `-e`    | **排除**指定的驱动类别，不安装指定类别的驱动。多个类别可重复指定。 |
  | `--missing-only`       | `-m`    | 仅安装未安装驱动的设备（即驱动缺失的设备）。              |
  | `--extract-path <目录>`  | `-x`    | 仅解压驱动到指定目录，不执行安装操作。默认解压到临时目录。       |
  | `--skip-verify`        | `-s`    | 跳过驱动索引文件验证，直接安装驱动。                  |
  | `--force`              | `-f`    | 强制安装，覆盖现有驱动。                        |

- 示例
    - `DriverIndexer.exe install D:\netcard`
    - `DriverIndexer.exe install D:\netcard.7z`
    - `DriverIndexer.exe install D:\netcard\*.7z`
    - `DriverIndexer.exe install D:\netcard.7z --index-path D:\netcard.json`
    - `DriverIndexer.exe install D:\netcard\*.7z --index-path D:\netcard\*.json`

### 安装离线系统驱动

安装离线系统驱动库中的驱动，未指定系统盘则全盘搜索系统盘。

`DriverIndexer.exe install-offline [系统盘路径]`

- 选项

| **参数**                 | **短参数** | **描述**                              |
|------------------------|---------|-------------------------------------|
| `--missing-only`       | `-m`    | 仅安装未安装驱动的设备（即驱动缺失的设备）。              |
| `--class <类别>`         | `-c`    | **包含**指定的驱动类别，仅安装匹配类别的驱动。多个类别可重复指定。 |
| `--exclude-class <类别>` | `-e`    | **排除**指定的驱动类别，不安装指定类别的驱动。多个类别可重复指定。 |

### 查看驱动索引信息

查看索引子命令，用于查看驱动索引文件中的信息。

`DriverIndexer.exe info <索引文件路径>`

- 示例
    - `DriverIndexer.exe info D:\netcard.json`

### 列举驱动

列出当前系统或离线系统的驱动存储中的驱动。

`DriverIndexer.exe list <系统盘路径>`

- 选项

| **参数**                   | **短参数** | **描述**                                |
|--------------------------|---------|---------------------------------------|
| `--system`               | `-s`    | 包含系统自带的驱动。                            |
| `--system-only`          | -       | 仅显示系统自带驱动。                            |
| `--class <驱动类别>`         | `-c`    | **包含**指定的驱动类别，仅显示指定类别驱动。多个类别可重复指定。    |
| `--exclude-class <驱动类别>` | `-e`    | **排除**指定的驱动类别，不显示指定类别驱动。多个类别可重复指定。    |
| `--provider <驱动供应商>`     | `-p`    | **包含**指定的驱动供应商，仅显示指定供应商驱动。多个供应商可重复指定。 |

- 示例
    - `DriverIndexer.exe list C:\`
    - `DriverIndexer.exe list C:\ --class net`
    - `DriverIndexer.exe list C:\ --exclude-class net`
    - `DriverIndexer.exe list C:\ --provider "Qualcomm, Inc."`

### 导入驱动

将驱动导入到系统驱动存储区。

`DriverIndexer.exe import <系统盘路径> <驱动路径>`

- 驱动路径格式：压缩包（限 7zip 所支持的格式）、目录格式。
- 支持通配符(`*`、`?`)，用于匹配多个驱动包。

- 选项

| **参数**            | **短参数** | **描述**             |
|-------------------|---------|--------------------|
| `--password <密码>` | `-p`    | 指定驱动包密码，用于解压驱动包。   |
| `--match-device`  | `-m`    | 匹配当前系统设备，默认匹配所有设备。 |

- 示例
    - `DriverIndexer.exe import C:\ D:\netcard.7z`
    - `DriverIndexer.exe import C:\ D:\netcard\*.7z`
    - `DriverIndexer.exe import C:\ D:\netcard.7z --password 123456`

### 导出驱动

从系统驱动存储中导出特定驱动。

`DriverIndexer.exe export <系统盘路径> <导出目录>`

- 选项

| **参数**                   | **短参数** | **描述**                                |
|--------------------------|---------|---------------------------------------|
| `--system`               | `-s`    | 包含系统自带的驱动。                            |
| `--system-only`          | -       | 仅导出系统自带驱动。                            |
| `--inf <inf文件名>`         | `-i`    | 指定驱动inf文件名，仅导出指定驱动。                   |
| `--class <驱动类别>`         | `-c`    | **包含**指定的驱动类别，仅导出指定类别驱动。多个类别可重复指定。    |
| `--exclude-class <驱动类别>` | `-e`    | **排除**指定的驱动类别，不导出指定类别驱动。多个类别可重复指定。    |
| `--provider <驱动供应商>`     | `-p`    | **包含**指定的驱动供应商，仅导出指定供应商驱动。多个供应商可重复指定。 |

- 示例
    - `DriverIndexer.exe export C:\ D:\drivers`
    - `DriverIndexer.exe export C:\ D:\drivers --inf netcard.inf`
    - `DriverIndexer.exe export C:\ D:\drivers --class net`
    - `DriverIndexer.exe export C:\ D:\drivers --exclude-class net`
    - `DriverIndexer.exe export C:\ D:\drivers --provider "Qualcomm, Inc."`

### 删除驱动

从系统驱动存储中删除驱动。

`DriverIndexer.exe remove <系统盘路径>`

- 选项

| **参数**               | **短参数** | **描述**                                |
|----------------------|---------|---------------------------------------|
| `--system`           | `-s`    | 包含系统自带的驱动。                            |
| `--system-only`      | -       | 仅删除系统自带驱动。                            |
| `--inf <inf文件名>`     | `-i`    | 指定驱动inf文件名，仅删除指定驱动。                   |
| `--class <驱动类别>`     | `-c`    | **包含**指定的驱动类别，仅删除指定类别驱动。多个类别可重复指定。    |
| `--provider <驱动供应商>` | `-p`    | **包含**指定的驱动供应商，仅删除指定供应商驱动。多个供应商可重复指定。 |
| `--all`              | `-a`    | 删除所有驱动。                               |

- 示例
    - `DriverIndexer.exe remove C:\ --inf netcard.inf`
    - `DriverIndexer.exe remove C:\ --class net`
    - `DriverIndexer.exe remove C:\ --provider "Qualcomm, Inc."`
    - `DriverIndexer.exe remove C:\ --all`

### 创建自解压驱动程序包

将 `DriverIndexer` 与驱动包合并，生成一个单一的 EXE 文件。这个 EXE 将自动运行，并按需解压和安装内置驱动。

`DriverIndexer.exe pack <驱动包/目录路径> <输出EXE路径>`

- 选项

| **参数**            | **短参数** | **描述**           |
|-------------------|---------|------------------|
| `--password <密码>` | `-p`    | 指定驱动包密码，用于加密驱动包。 |

- 示例
    - `DriverIndexer.exe pack D:\netcard D:\netcard.exe`
    - `DriverIndexer.exe pack D:\netcard.7z D:\netcard.exe`

### 驱动文件整理

将文件夹中的驱动按驱动类别、厂商等规则分类，并根据 INF 文件中的信息进行重命名。

`DriverIndexer.exe organize <驱动路径> <导出目录>`

- 示例
    - `DriverIndexer.exe organize D:\netcard D:\netcard-organized`

### 密码加密

为了保护驱动包密码，提供密码加密功能。加密后的密码可以在安装驱动包时指定 `--password <加密后的密码>`
参数使用。密码加密使用`AES-128`算法，确保密码安全性。

`DriverIndexer.exe encrypt <密码>`

- 示例
    - `DriverIndexer.exe encrypt 12345678`
    - `DriverIndexer.exe install D:\netcard.7z --password enc:XXXXX`

### 全局选项

`DriverIndexer.exe [全局选项] 命令 [参数] [选项]`

| 参数              | 短参数 | 描述                                               | 默认值  |
|-----------------|-----|--------------------------------------------------|------|
| `--debug`       | 无   | 调试模式，输出调试信息到控制台                                  | 无    |
| `--language`    | 无   | 设置程序语言 (`En`, `zh-cn`, `zh-tw`, `ja-jp`、`ko-kr`) | 自动识别 |
| `--log<日志文件路径>` | 无   | 开启日志。将所有运行信息打印到指定文件中，方便排查问题。                     | 无    |

### 驱动类别参考

以下是常见的驱动类别名称，可用于`--class`参数、`--exclude-class`参数指定驱动类别。

> 注意：
>
> - 驱动类别名称不区分大小写，如`Display`和`display`效果相同。
> - 驱动类别名称可以由驱动厂商定义，故没有限制驱动类别名称，请确保类别名称正确。

| 类别名称        | 描述              |
|-------------|-----------------|
| Display     | 显示适配器           |
| Net         | 网络适配器           |
| HID         | 人体学输入设备         |
| USB         | USB 控制器         |
| Bluetooth   | 蓝牙设备            |
| Printer     | 打印机             |
| Imaging     | 图像设备            |
| SCSIAdapter | SCSI 和 RAID 控制器 |
| DiskDrive   | 磁盘驱动器           |
| Computer    | 计算机             |
| Processor   | 处理器             |
| Monitor     | 监视器             |
| Keyboard    | 键盘              |
| Pointer     | 鼠标和其他指针设备       |
| Modem       | 调制解调器           |
| Media       | 多媒体设备           |
| System      | 系统设备            |

## 编译

### 环境要求

- Rust 1.65 或以上版本
- VC-LTL
- YY-Thunks

### 配置项目

新建 `.env` 文件，配置以下内容：

```env
# 密钥，用于密码加密
# 此处密钥为示例，建议自定义密钥
SECRET_KEY = 0123456789ABCDEF0123456789ABCDEF
```

### 编译项目

使用 `cargo build --release` 命令编译项目。编译完成后，在 `target/release` 目录下即可找到 `DriverIndexer.exe` 文件。

## 开源许可

`DriverIndexer` 使用 GPL V3.0 协议开源，请尽量遵守开源协议。

## 致谢

- Hydrogen
- Lightning
- Skyfree
- 红毛樱木
- 小鸭子
- 毛利
- 优捷易

## 参与贡献

1. Fork 本仓库
2. 新建 Feat_xxx 分支
3. 提交代码
4. 新建 Pull Request
