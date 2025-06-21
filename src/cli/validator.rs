use crate::i18n::getLocaleText;
use std::path::{Path, PathBuf};

/// 是否为有效的路径
pub fn isValidPath(path: &str) -> Result<(), String> {
    if !PathBuf::from(path).exists() {
        return Err(getLocaleText("path-not-exist", None));
    };
    Ok(())
}

/// 是否为有效的目录路径
pub fn isValidDirectory(directory: &str) -> Result<(), String> {
    let path = PathBuf::from(directory);
    if !path.exists() {
        return Err(getLocaleText("dir-not-exist", None));
    };
    if !path.is_dir() {
        return Err(getLocaleText("path-not-exist", None));
    };
    Ok(())
}

/// 是否为系统路径
pub fn isValidSystemPath(systemPath: &str) -> Result<(), String> {
    let path = Path::new(&systemPath);
    if !path.exists() {
        return Err(getLocaleText("path-not-exist", None));
    };
    if !path.join(r"Windows\System32\ntoskrnl.exe").exists() {
        return Err(getLocaleText("not-system-path", None));
    }
    Ok(())
}

/// 是否为有效的路径（包括通配符）
pub fn isValidPathIncludeWildcard(path: &str) -> Result<(), String> {
    let path = PathBuf::from(path);

    let fileName = path.file_name().unwrap().to_str().unwrap();
    if fileName.contains('*') || fileName.contains('?') {
        return if path.parent().unwrap().exists() {
            Ok(())
        } else {
            Err(getLocaleText("path-not-exist", None))
        };
    }
    if !path.exists() {
        return Err(getLocaleText("path-not-exist", None));
    }
    Ok(())
}

/// 是否为有效的驱动类别
pub fn isValidDriverClass(class: &str) -> Result<(), String> {
    // HKEY_LOCAL_MACHINE\SYSTEM\CurrentControlSet\Control\Class\{f8ecafa6-66d1-41a5-899b-66585d7216b7}
    let driverClass = [
        "XboxComposite",
        "RDPDR",
        "DXGKrnl",
        "RemotePosDevice",
        "DigitalMediaDevices",
        "PrintQueue",
        "WCEUSBS",
        "SecurityAccelerator",
        "HidMsr",
        "SystemRecovery",
        "vhdmp",
        "fvevol",
        "fvevol",
        "USB",
        "ContentScreener",
        "Media Center Extender",
        "PnpPrinters",
        "Dot4",
        "Replication",
        "Dot4Print",
        "CDROM",
        "Computer",
        "DiskDrive",
        "Display",
        "FDC",
        "HDC",
        "Keyboard",
        "MEDIA",
        "Modem",
        "Monitor",
        "Mouse",
        "MTD",
        "MultiFunction",
        "Net",
        "NetClient",
        "NetService",
        "NetTrans",
        "PCMCIA",
        "Ports",
        "Printer",
        "SCSIAdapter",
        "System",
        "Unknown",
        "FloppyDisk",
        "HidLineDisplay",
        "Processor",
        "MultiPortSerial",
        "Memory",
        "SmartCardReader",
        "Sensor",
        "VolumeSnapshot",
        "SmrDisk",
        "ScmDisk",
        "SmrVolume",
        "ScmVolume",
        "Biometric",
        "Proximity",
        "AudioProcessingObject",
        "OposLegacyDevice",
        "SoftwareComponent",
        "FSFilterSystem",
        "XLGuard",
        "SoftwareDevice",
        "PerceptionSimulation",
        "PhysicalQuotaManagement",
        "1394",
        "Infrared",
        "Image",
        "TapeDrive",
        "BasicDisplay",
        "Volume",
        "ContinuousBackup",
        "Battery",
        "HIDClass",
        "HidCashDrawer",
        "61883",
        "RdpVideoMiniport",
        "QuotaManagement",
        "NetDriver",
        "TS_Generic",
        "USBDevice",
        "CopyProtection",
        "LegacyDriver",
        "SmartCard",
        "EhStorSilo",
        "XLWFP",
        "SDHost",
        "Encryption",
        "rdpbus",
        "AntiVirus",
        "RDCamera",
        "ActivityMonitor",
        "USBFunctionController",
        "AVC",
        "AudioEndpoint",
        "BarcodeScanner",
        "WSDPrintDevice",
        "POSPrinter",
        "Camera",
        "RDPDR",
        "CFSMetadataServer",
        "MediumChanger",
        "SecurityEnhancer",
        "Miracast",
        "SBP2",
        "HSM",
        "Holographic",
        "XnaComposite",
        "SecurityDevices",
        "SmartCardFilter",
        "Bluetooth",
        "Extension",
        "Infrastructure",
        "UCM",
        "WPD",
        "ComputeAccelerator",
        "Firmware",
        "Compression",
        "Virtualization",
        "OpenFileBackup",
        "Undelete",
    ];
    for item in driverClass.iter() {
        if item.to_lowercase() == class.to_lowercase() {
            return Ok(());
        }
    }
    Err(getLocaleText("not-driver-category", None))
}
