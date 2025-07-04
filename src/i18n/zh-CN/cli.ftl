# template
-opening-brace = {"{"}
-closing-brace = {"}"}

template =
 {-opening-brace}bin{-closing-brace} V{-opening-brace}version{-closing-brace}

 用法: {-opening-brace}bin{-closing-brace}.exe [选项] [命令] [参数]

 命令:
  {-opening-brace}subcommands{-closing-brace}

 选项:
  {-opening-brace}options{-closing-brace}

help-message = 打印帮助信息
version-message = 打印版本信息

help = 打印帮助信息。显示此信息或指定命令帮助

on-debug = 开启调试模式
opened-debug = 调试模式已打开，日志保存在 { $path }

# 子命令

## create-index
create-index = 创建驱动索引。索引格式：JSON
save-index-path = 索引文件保存位置

## load-driver
load-driver = 安装匹配驱动。自动匹配压缩包中的驱动程序，解压并安装
package-path = 压缩包路径
index-path = 索引文件路径
package-password = 设置压缩包密码
match-all-device = 匹配所有设备
driver-category = 设置安装的驱动程序类别
only-unzip = 仅解压驱动程序而不安装
offline-import = 离线导入驱动
eject-driver-cd = 弹出免驱设备虚拟光驱以识别真实USB设备

## load-offline-driver
load-offline-driver = 加载离线系统驱动

## import-driver
import-driver = 导入驱动。支持在线系统、离线系统
match-device = 匹配本机设备

## export-driver
export-driver = 导出驱动。支持在线系统、离线系统
driver-name = 驱动名称

## remove-driver
remove-driver = 删除驱动。支持在线系统、离线系统

## classify-driver
classify-driver = 整理驱动程序
rename-driver = 重命名驱动父目录

## create-driver
create-driver = 创建驱动包程序。打包程序与驱动包，便于分发
driver-package-program-path = 驱动包程序路径

## scan-devices
scan-devices = 扫描硬件设备更改
scan-devices-success = 扫描硬件设备更改成功
scan-devices-failed = 扫描硬件设备更改失败

# 验证器
path-not-exist = 路径不存在，请确保输入的目录存在
dir-not-exist = 目录不存在，请确保输入的目录存在
not-driver-category = 驱动程序类别不正确，请输入正确的驱动程序类别
not-system-path = 无效的系统盘，请确保输入的盘符存在操作系统
