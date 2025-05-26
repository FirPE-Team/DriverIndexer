Info = 信息
Success = 成功
Warning = 警告
Err = 错误

temp-create-failed=临时目录创建失败
temp-remove-failed=临时目录删除失败

# 创建索引
processing = 正在处理中，请稍候······
no-hardware = 未检测到此文件中的硬件ID: { $path }
inf-parsing-err = INF解析错误: { $path }
index-save-failed = 索引文件保存失败
no-inf-find = 没有找到驱动信息
total-info = 总 {$total} 个项目，已处理 {$success} 个项目，{$error} 个项目无法处理，{$blankCount} 个项目可能没有硬件ID信息
saveInfo = 驱动索引保存在 { $path }

# 加载驱动
load-driver-package = 加载驱动包: { $path }
no-device = 获取硬件信息失败
unzip-index-failed = 无法解压索引文件，请确认压缩包中是否存在索引文件
index-parsing-failed = 索引文件解析失败，请重新生成索引文件
no-driver-package = 在驱动程序包中未检测到驱动程序
no-found-driver-currently = 找不到当前需要安装驱动的设备
install-message =
 设备: {$class} { $deviceName }
             硬件ID: { $deviceID }
             驱动: { $driver }, 版本: { $version }
driver-unzip-failed = 驱动程序解压失败
driver-unzip-success = 驱动程序解压成功
driver-install-failed = 驱动程序安装失败
driver-install-success = 驱动程序安装成功

# 加载离线驱动
load-offline-driver = 加载离线系统驱动: { $path }
not-found-offline-system = 没有找到离线系统

# 导入驱动
offline-Arch-Err = 获取离线系统架构失败
driver-import-success = 驱动导入成功: { $inf }
driver-import-failed = 驱动导入失败: { $inf }
driver-import-summary = 共处理 { $total } 个驱动，成功 { $success }，失败 { $fail }

## 删除驱动
driver-remove = 删除驱动: { $inf }
driver-remove-success = 驱动删除成功
driver-remove-failed = 驱动删除失败

# 整理驱动
Drivers-finishing-complete = 驱动整理成功

# 创建驱动包程序
Driver-finishing-create = 驱动包程序创建成功
Pack-Driver-failed = 创建驱动包程序失败

# 驱动类别
# ADAPTER = 适配器
# BATTERY = 电池
# CDROM = CDROM驱动器
# COMPUTER = 计算机
# DECODER = 解码器
# DISKDRIVE = 磁盘驱动器
# DISPLAY = 显卡
# FDC = 软盘控制器
# FLOPPYDISK = 软盘驱动器
# HDC = 磁盘控制器
# INFRARED = 红外适配器
# KEYBOARD = 键盘
# LEGACYDRIVER = 非即插即用设备
# MEDIA = 声卡
# MODEM = 调制解调器
# MONITOR = 监视器
# MOUSE = 鼠标
# NET = 网卡
# PORTS = 端口
# PRINTER = 打印机
# SCSIADAPTER = SCSI控制器
# SMARTCARDREADER = 智能卡阅读器
# MTD = 内存类设备
# SYSTEM = 系统设备
# BLUETOOTH = 蓝牙
# BluetoothAuxiliary = 蓝牙辅助
# PROCESSOR = 处理器
# USB = 通用串行总线
# HIDClass = 人体学输入设备
# Image = 图像设备
# Volume = 存储卷
# APMSUPPORT = APM支持设备
# GPS = 全球卫星定位设备
# MediumChanger(MEDIUM) = 媒体更换器设备
# MultiFunction = 多功能适配器
# MultiPortSerial = 多串口适配器
# NetClient = 网络客户端
# NetService = 网络服务
# NetTrans = 网络协议
# NODRIVER = 无驱动设备
# PCMCIA = PCMCIA适配器
# PRINTERUPGRADE = 复合型打印机
# TapeDrive = 磁带驱动器
# Unknown = 其它设备
# AVC = AVC设备
# BiometricDevice(BIOMETRIC) = 生物识别设备
# 1394 = IEEE 1394总线主控制器
# DOT4 = IEEE 1284.4设备
# Dot4Print = IEEE 1284.4兼容打印机
# PnpPrinters = IEEE 1394 和SCSI 打印机
# SBP2 = SBP2 IEEE 1394设备
# 1394DEBUG = 1394DEBUG
# ENUM1394 = ENUM1394
# 61883 = 61883设备
# SECURITYACCELERATOR = 安全加速器
# VolumeSnapshot = 存储卷卷影副本
# WCEUSBS = 移动设备
# FSFILTER = FS筛选器
# INFINIBAND = 无限带宽
