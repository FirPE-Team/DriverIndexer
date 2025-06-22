Info = Info{"   "}
Success = Success
Warning = Warning
Err = Error{"  "}

temp-create-failed=Temporary directory create failed
temp-remove-failed=Temporary directory deletion failed

# create-index
processing = In process, please wait……
no-hardware = The hardware id in this file is not detected: { $path }
inf-parsing-err = INF parsing error: { $path }
index-save-failed = Failed to save index file
no-inf-find = No driver information found
total-info = Total {$total} items，Processed {$success} items，{$error} items failed to process，{$blankCount} items may not have hardware id information
saveInfo = The drive index is saved in { $path }

# load-driver
load-driver-package = Load driver package: { $path }
no-device = Failed to obtain hardware information
load-index = load index: {$path}
unzip-index-failed = Failed to unzip the index file, please confirm whether the index file exists in the compressed package
index-parsing-failed = Index file parsing failed, Please regenerate the index file
no-driver-package = No driver detected in the driver package
no-found-driver-currently = Cannot find the device that currently needs to install the driver
install-message =
 Device: {$class} { $deviceName }
            HardwareID: { $deviceID }
            Drive: { $driver }, Version: { $version }
driver-unzip-failed = Driver unzip failed
driver-unzip-success = Driver unzip successfully
driver-install-failed = Driver installation failed
driver-install-success = Driver installed successfully
ejecting-driver-cd = Eject virtual optical drive without drive device ({ $drive })

# load-offline-driver
loading-offline-driver = Load offline system driver ({ $path })
not-found-offline-system = No offline systems found

# import-driver
offline-Arch-Err = get offline System arch failed
driver-import-success = Driver imported successfully: { $inf }
driver-import-failed = Driver import failed: { $inf }
driver-import-summary = Processed { $total } drivers, success { $success }, failure { $fail }

# export-driver
driver-export-success = Driver export successfully
driver-export-failed = Driver export failed

# remove-driver
driver-remove-success = Driver remove successfully
driver-remove-failed = Driver remove successfully

# classify-driver
Drivers-finishing-complete = Drivers finishing complete
Drivers-finishing-failed = Drivers finishing failed

# create-driver
Driver-finishing-create = Driver package program created successfully
Pack-Driver-failed = Driver package creation failed

# Drive category
# ADAPTER = ADAPTER
# BATTERY = BATTERY
# CDROM = CDROM
# COMPUTER = COMPUTER
# DECODER = DECODER
# DISKDRIVE = DISKDRIVE
# DISPLAY = DISPLAY
# FDC = FDC
# FLOPPYDISK = FLOPPYDISK
# HDC = HDC
# INFRARED = INFRARED
# KEYBOARD = KEYBOARD
# LEGACYDRIVER = LEGACYDRIVER
# MEDIA = MEDIA
# MODEM = MODEM
# MONITOR = MONITOR
# MOUSE = MOUSE
# NET = NET
# PORTS = PORTS
# PRINTER = PRINTER
# SCSIADAPTER = SCSIADAPTER
# SMARTCARDREADER = SMARTCARDREADER
# MTD = MTD
# SYSTEM = SYSTEM
# BLUETOOTH = BLUETOOTH
# BluetoothAuxiliary = BluetoothAuxiliary
# PROCESSOR = PROCESSOR
# USB = USB
# HIDClass = HIDClass
# Image = Image
# Volume = Volume
# APMSUPPORT = APMSUPPORT
# GPS = GPS
# MediumChanger(MEDIUM) = MediumChanger(MEDIUM)
# MultiFunction = MultiFunction
# MultiPortSerial = MultiPortSerial
# NetClient = NetClient
# NetService = NetService
# NetTrans = NetTrans
# NODRIVER = NODRIVER
# PCMCIA = PCMCIA
# PRINTERUPGRADE = PRINTERUPGRADE
# TapeDrive = TapeDrive
# Unknown = Unknown
# AVC = AVC
# BiometricDevice(BIOMETRIC) = BiometricDevice(BIOMETRIC)
# 1394 = 1394
# DOT4 = DOT4
# Dot4Print = Dot4Print
# PnpPrinters = PnpPrinters
# SBP2 = SBP2
# 1394DEBUG = 1394DEBUG
# ENUM1394 = ENUM1394
# 61883 = 61883
# SECURITYACCELERATOR = SECURITYACCELERATOR
# VolumeSnapshot = VolumeSnapshot
# WCEUSBS = WCEUSBS
# FSFILTER = FSFILTER
# INFINIBAND = INFINIBAND
