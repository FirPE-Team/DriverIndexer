# DriverIndexer

[简体中文](README.zh.md) [English](README.md)

## 简介

`DriverIndexer` 是用于创建、读取和安装驱动包索引的工具。

- ⚡ **按需解压** - 仅解压当前设备匹配的驱动，大幅提升安装速度
- 🎯 **智能匹配** - 自动识别硬件并匹配合适的驱动程序
- 📦 **多格式支持** - 支持目录、7z压缩包等多种驱动包格式
- 🔄 **离线支持** - 支持离线系统驱动管理，不依赖DISM环境
- 🖥️ **命令行界面** - 可静默安装，便于自动化部署
- 🛠️ **驱动管理** - 导入、导出、删除系统驱动程序
- 🔧 **驱动打包** - 将驱动与程序合并为单一可执行文件

### `DriverIndexer`有什么用？

很多人为了安装方便，将多个驱动打包为一个驱动包，而一般安装驱动包需要全部解压，再调用`Dpinst`等工具进行安装驱动，这种方法非常消耗时间与性能。
`DriverIndexer`的功能就是按需解压当前匹配的驱动，并自动安装。

### 索引文件是什么？

由于硬件ID存储在INF文件内，按需解压需要建立 `INF文件中硬件ID列表` 与 `驱动包内驱动所在路径`
两者的对应关系，这一对应关系我们称之为`索引`。根据索引就能确定设备所需要安装的驱动，进而进行按需解压驱动并安装。

### 为什么索引文件使用`JSON`格式？

通常情况下，一个驱动包内的索引不会超过10MB，而这个大小的数据量使用通用的`JSON`格式足够了。

### 为什么可以不指定索引文件来安装驱动？

当不指定索引文件时，`DriverIndexer`会解压驱动包中的所有INF文件，即时建立索引，最后根据索引的信息来匹配驱动。

### 与 EasyDrv/驱动总裁 有何区别？

`DriverIndexer`是命令行程序，这意味着可以静默安装驱动，不需要进行界面交互，使得体验与内置驱动一样。

### 从哪里获取驱动包？

> 我们更提倡自己下载、搜集驱动包，如有需求也可自行提取目前各个驱动软件内的驱动包（一般此类驱动包无版权）

以下为推荐的驱动包下载网站（均免费、无加密）

- [SamDrivers](https://driveroff.net)
- [DriverPack](https://drp.su/en/foradmin?_blank)
- [3DP](https://www.3dpchip.com/3dpchip/3dp/net_down.php?_blank)
- [DriverOff](https://driveroff.net/category/dp?_blank)
- [BatPEDriver](http://forum.ru-board.com/topic.cgi?forum=62&topic=24098&start=71&limit=1&m=1#1?_blank)

## 软件架构

使用`Rust`编写，调用`Devcon.exe`获取硬件信息，使用WindowsAPI安装设备驱动。

### 驱动匹配规则

1. 默认仅匹配未安装驱动的设备
2. 专用驱动优先级大于公版
3. 高版本优先级大于低版本
4. 三次匹配（防止部分驱动未安装成功）

## 使用说明

本程序为命令行程序，故需要在其后面接参数运行，如直接双击程序将会出现“闪退”现象，您可通过`cmd`、`PowerShell`等终端来运行。  
注意：请使用**管理员身份**运行终端。

### 全局选项

`DriverIndexer.exe [全局选项] 命令 参数`

- `--language <语言>`：指定程序语言。可选值为`en`（英文）、`zh-CN`（简体中文）、`zh-TW`（繁体中文）。默认值为系统语言。
- `--debug`：开启调试模式。打印更多调试信息，方便排查问题。
- `--log <日志文件路径>`：开启日志模式。将所有运行信息打印到指定文件中，方便排查问题。
- `--help`：查看帮助信息。

### 创建索引

创建索引子命令，用于创建驱动包索引。

`DriverIndexer.exe create-index <驱动路径> <索引保存路径>`
`DriverIndexer.exe index <驱动路径> <索引保存路径>`

- 选项
    - `--password <解压密码>`：指定驱动包密码

- 示例
    - `DriverIndexer.exe create-index D:\netcard.7z D:\index.json`
    - `DriverIndexer.exe create-index D:\netcard D:\index.json`

### 查看索引信息

查看索引子命令，用于查看索引文件中的信息。

`DriverIndexer.exe index-info <索引路径>`
`DriverIndexer.exe info <索引路径>`

### 安装驱动

安装驱动子命令，用于安装驱动包中的驱动。支持压缩包（限7zip所支持的格式）与目录格式。

`DriverIndexer.exe install-driver <驱动路径> [-p 解压密码] [--AllDevice] [--ExtractDriver] [--class 驱动类别]`
`DriverIndexer.exe install <驱动路径> [-p 解压密码] [--AllDevice] [--ExtractDriver] [--class 驱动类别]`

- 选项
    - `--password <解压密码>`：指定驱动包密码
    - `--class <驱动类别>`：指定驱动类别
    - `--match_device`：匹配当前系统设备
    - `--AllDevice`：匹配所有设备，默认仅安装未安装驱动的设备
    - `--ExtractDriver <解压目录>`：仅解压驱动，不安装驱动

- 示例
    - 无驱动索引: `DriverIndexer.exe install-driver <驱动路径>`
        - `DriverIndexer.exe install-driver D:\netcard`
        - `DriverIndexer.exe install-driver D:\netcard.7z`
        - `DriverIndexer.exe install-driver D:\netcard\*.7z`
    - 有驱动索引: `DriverIndexer.exe install-driver <驱动路径> <索引路径>`
        - `DriverIndexer.exe install-driver D:\netcard.7z netcard.json`
        - `DriverIndexer.exe install-driver D:\netcard\*.7z D:\netcard\*.json`

### 安装离线系统驱动

安装离线系统驱动库中的驱动，未指定系统盘则全盘搜索系统盘。默认仅安装未安装驱动的设备。

`DriverIndexer.exe install-offline-driver [系统盘路径]`

- 选项
    - `--all-Device`：匹配所有设备
    - `--class <驱动类别>`：仅安装指定类别驱动

### 列举驱动

列举系统中所有驱动程序，支持在线系统与离线系统。

`DriverIndexer.exe list-driver <系统盘路径>`
`DriverIndexer.exe list <系统盘路径>`

- 选项
    - `--class <驱动类别>`：指定驱动类别
    - `--provider <驱动供应商>`：指定驱动供应商

### 导入驱动

将驱动包中的所有驱动程序导入系统中，支持在线系统与离线系统。

`DriverIndexer.exe import-driver <系统盘路径> <驱动路径>`
`DriverIndexer.exe import <系统盘路径> <驱动路径>`

- 选项
    - `--password <解压密码>`：指定驱动包密码
    - `--match-device`: 匹配当前系统设备

### 导出驱动

将系统中所有驱动程序导出到指定目录，支持在线系统与离线系统。

`DriverIndexer.exe export-driver <系统盘路径> <导出目录>`
`DriverIndexer.exe export <系统盘路径> <导出目录>`

- 选项
    - `--inf <驱动名称>`：指定驱动名称
    - `--class <驱动类别>`：指定驱动类别
    - `--provider <驱动供应商>`：指定驱动供应商

### 删除驱动

删除系统中的驱动程序，支持在线系统与离线系统。

`DriverIndexer.exe remove-driver <系统盘路径>`

- 选项
    - `--inf <驱动名称>`：指定驱动名称
    - `--class <驱动类别>`：指定驱动类别
    - `--provider <驱动供应商>`：指定驱动供应商

### 整理驱动

整理指定目录中的驱动程序，按驱动类别、供应商进行分类。

`DriverIndexer.exe organize-driver 驱动路径`

- `DriverIndexer.exe organize-driver D:\netcard`

### 打包驱动包程序

将`DriverIndexer`与驱动包合并，生成exe二进制可执行文件，生成的可执行文件将自动读取自身驱动包，仅解压所需驱动(避免二次解压)。

> 温馨提示：驱动包不能设置密码，否则会导致驱动安装失败。

`DriverIndexer.exe create-driver <驱动路径> <输出路径>`

- 示例
    - 从文件中创建程序驱动包
        - `DriverIndexer.exe create-driver D:\netcard.7z D:\netcard.exe`
    - 从目录中创建程序驱动包
        - `DriverIndexer.exe create-driver D:\netcard D:\netcard.exe`

## 驱动类别参考

以下是常见的驱动类别名称，可用于`--class`参数：

> 注意：
> - 驱动类别名称不区分大小写，如`Display`和`display`效果相同。
> - 驱动类别名称可以由驱动厂商定义，故没有限制驱动类别名称，请确保类别名称正确。

| 类别名称        | 描述           |
|-------------|--------------|
| Display     | 显示适配器        |
| Net         | 网络适配器        |
| Media       | 声音、视频和游戏控制器  |
| System      | 系统设备         |
| HID         | 人体学输入设备      |
| USB         | USB控制器       |
| Bluetooth   | 蓝牙设备         |
| Printer     | 打印机          |
| Imaging     | 图像设备         |
| SCSIAdapter | SCSI和RAID控制器 |
| DiskDrive   | 磁盘驱动器        |
| Computer    | 计算机          |
| Processor   | 处理器          |
| Monitor     | 监视器          |
| Keyboard    | 键盘           |
| Pointer     | 鼠标和其他指针设备    |
| Modem       | 调制解调器        |
| Media       | 多媒体设备        |
| System      | 系统设备         |

## 开源许可

`DriverIndexer` 使用 GPL V3.0 协议开源，请尽量遵守开源协议。

## 致谢

- Hydrogen
- Lightning
- Skyfree
- 红毛樱木
- 小鸭子
- 毛利

## 参与贡献

1. Fork 本仓库
2. 新建 Feat_xxx 分支
3. 提交代码
4. 新建 Pull Request
