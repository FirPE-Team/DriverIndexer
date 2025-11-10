use clap::{Parser, ValueEnum};
use rust_i18n::t;
use std::path::PathBuf;

// 主程序参数
#[derive(Parser, Debug)]
#[clap(version)]
// #[clap(disable_version_flag = true)]
pub struct Cli {
    #[clap(subcommand)]
    pub command: Command,

    /// 设置程序语言
    #[clap(help = t!("options.lang").to_string())]
    #[clap(long, value_enum)]
    pub(crate) language: Option<Language>,

    /// 输出日志文件
    #[clap(short('L'), long("log"), help(t!("options.log-path").to_string()))]
    pub log_path: Option<PathBuf>,

    /// 开启调试模式
    #[clap(short('D'), long("debug"), help(t!("options.debug").to_string()))]
    pub debug: bool,
    // 版本信息
    // #[clap(short('V'), long("version"), help(t!("options.version").to_string()))]
    // #[arg(short = 'v',long = "version",action = ArgAction::Version,help = "Print version")]
    // version: bool,
}

#[derive(Debug, Clone, ValueEnum)]
pub enum Language {
    /// English language
    En,
    /// Simplified Chinese
    ZhCn,
    /// Traditional Chinese
    ZhTw,
}

#[derive(Parser, Debug)]
pub enum Command {
    /// 创建索引子命令
    #[clap(aliases = &["index"], about(t!("commands.create-index").to_string()))]
    CreateIndex {
        /// 驱动文件路径
        #[clap(value_parser = exist_file_wildcard_parser)]
        #[clap(help(t!("options.package-path").to_string()))]
        driver_path: PathBuf,

        /// 索引文件路径
        #[clap(help(t!("options.save-index-path").to_string()))]
        index_path: Option<PathBuf>,

        /// 驱动密码
        #[clap(short, long)]
        #[clap(help(t!("options.package-password").to_string()))]
        password: Option<String>,
    },

    /// 索引信息子命令
    #[clap(aliases = &["info"], about(t!("commands.index-info").to_string()))]
    IndexInfo {
        /// 索引文件路径
        #[clap(help(t!("options.index-path").to_string()))]
        index_path: PathBuf,
    },

    /// 加载驱动子命令
    #[clap(aliases = &["install"], about(t!("commands.install-driver").to_string()))]
    InstallDriver {
        /// 驱动文件路径
        #[clap(value_parser = exist_file_wildcard_parser)]
        #[clap(help(t!("options.package-path").to_string()))]
        driver_path: PathBuf,

        /// 索引文件路径
        #[clap(short, long)]
        #[clap(help(t!("options.index-path").to_string()))]
        index_path: Option<PathBuf>,

        /// 驱动密码
        #[clap(short, long)]
        #[clap(help(t!("options.package-password").to_string()))]
        password: Option<String>,

        /// 是否匹配所有设备，默认仅匹配没有安装驱动的设备
        #[clap(short = 'a', long)]
        #[clap(help(t!("options.match-all-device").to_string()))]
        match_all: bool,

        /// 驱动类别
        #[clap(short, long)]
        #[clap(help(t!("options.driver-category").to_string()))]
        class: Option<String>,

        /// 是否仅解压驱动文件
        #[clap(short = 'x', long)]
        #[clap(help(t!("options.only-unzip").to_string()))]
        extract_path: Option<PathBuf>,

        /// 是否 ejection 驱动 CD
        #[clap(short, long, help(t!("commands.eject-virtual-drive").to_string()))]
        eject_virtual_driver: bool,

        /// 是否强制加载驱动
        #[clap(short, long)]
        #[clap(help(t!("options.force").to_string()))]
        force: bool,
    },

    /// 加载离线驱动子命令
    #[clap(about(t!("commands.install-offline-driver").to_string()))]
    InstallOfflineDriver {
        /// 系统盘符
        #[clap(value_parser = exist_system_parser)]
        #[clap(help(t!("options.system-drive").to_string()))]
        system_drive: Option<PathBuf>,

        /// 是否匹配所有设备
        #[clap(short = 'a', long)]
        #[clap(help(t!("options.match-all-device").to_string()))]
        match_all: bool,

        /// 驱动类别
        #[clap(short, long)]
        #[clap(help(t!("options.driver-category").to_string()))]
        class: Option<String>,
    },

    /// 导入驱动子命令
    #[clap(aliases = &["import"], about(t!("commands.import-driver").to_string()))]
    ImportDriver {
        /// 系统盘符
        #[clap(value_parser = exist_system_parser)]
        #[clap(help(t!("options.system-drive").to_string()))]
        system_drive: PathBuf,

        /// 驱动文件路径
        #[clap(value_parser = exist_file_wildcard_parser)]
        #[clap(help(t!("options.package-path").to_string()))]
        driver_path: PathBuf,

        /// 驱动密码
        #[clap(short, long)]
        #[clap(help(t!("options.package-password").to_string()))]
        password: Option<String>,

        /// 是否匹配所有设备
        #[clap(short, long)]
        #[clap(help(t!("options.match-device").to_string()))]
        match_all: bool,
    },

    /// 导出驱动子命令
    #[clap(aliases = &["export"], about(t!("commands.export-driver").to_string()))]
    ExportDriver {
        /// 系统盘符
        #[clap(index = 1, value_parser = exist_system_parser)]
        #[clap(help(t!("options.system-drive").to_string()))]
        system_drive: PathBuf,

        /// 导出路径
        #[clap(index = 2)]
        #[clap(help(t!("options.export-path").to_string()))]
        export_path: PathBuf,

        /// 指定驱动名称
        #[clap(short, long)]
        #[clap(help(t!("options.driver-name").to_string()))]
        inf: Option<String>,

        /// 指定驱动类别
        #[clap(short, long)]
        #[clap(help(t!("options.driver-category").to_string()))]
        class: Option<String>,

        /// 指定驱动厂商
        #[clap(short, long)]
        #[clap(help(t!("options.driver-provider").to_string()))]
        provider: Option<String>,
    },

    /// 删除驱动子命令
    #[clap(aliases = &["remove"], about(t!("commands.remove-driver").to_string()))]
    #[clap(group(
        clap::ArgGroup::new("remove_filter")
            .required(true)
            .args(&["inf", "class", "provider", "all"])
            .multiple(true)
    ))]
    RemoveDriver {
        /// 系统盘符
        #[clap(index = 1, value_parser = exist_system_parser)]
        #[clap(help(t!("options.system-drive").to_string()))]
        system_drive: PathBuf,

        /// 指定驱动名称
        #[clap(short, long)]
        #[clap(help(t!("options.driver-name").to_string()))]
        inf: Option<String>,

        /// 指定驱动类别
        #[clap(short, long)]
        #[clap(help(t!("options.driver-category").to_string()))]
        class: Option<String>,

        /// 指定驱动厂商
        #[clap(short, long)]
        #[clap(help(t!("options.driver-provider").to_string()))]
        provider: Option<String>,

        /// 是否删除所有驱动
        #[clap(short, long)]
        #[clap(help(t!("options.remove-all-driver").to_string()))]
        all: bool,
    },

    /// 列举驱动子命令
    #[clap(aliases = &["list"], about(t!("commands.list-driver").to_string()))]
    ListDriver {
        /// 系统盘符
        #[clap(index = 1, value_parser = exist_system_parser)]
        #[clap(help(t!("options.system-drive").to_string()))]
        system_drive: PathBuf,

        /// 指定驱动类别
        #[clap(short, long)]
        #[clap(help(t!("options.driver-category").to_string()))]
        class: Option<String>,

        /// 指定驱动厂商
        #[clap(short, long)]
        #[clap(help(t!("options.driver-provider").to_string()))]
        provider: Option<String>,
    },

    /// 整理驱动子命令
    #[clap(aliases = &["organize"], about(t!("commands.organize-driver").to_string()))]
    OrganizeDriver {
        /// 驱动文件路径
        #[clap(index = 1, value_parser = exist_dir_parser)]
        #[clap(help(t!("options.drive-path").to_string()))]
        drive_path: PathBuf,

        /// 导出路径
        #[clap(index = 2)]
        #[clap(help(t!("options.export-path").to_string()))]
        export_path: PathBuf,

        /// 是否重命名驱动文件
        #[clap(short, long)]
        #[clap(help(t!("options.rename").to_string()))]
        rename: bool,
    },

    /// 创建驱动程序包子命令
    #[clap(aliases = &["pack"], about(t!("commands.pack-driver").to_string()))]
    PackDriver {
        /// 驱动文件路径
        #[clap(index = 1, value_parser = exist_dir_parser)]
        #[clap(help(t!("options.drive-path").to_string()))]
        drive_path: PathBuf,

        /// 驱动程序路径
        #[clap(index = 2)]
        #[clap(help(t!("options.driver-package-program-path").to_string()))]
        program_path: PathBuf,
    },

    /// 扫描设备硬件更改子命令
    #[clap(aliases = &["scan"], about(t!("commands.scan-devices").to_string()))]
    ScanDevices,

    /// 弹出免驱设备子命令
    #[clap(aliases = &["eject"], about(t!("commands.eject-virtual-drive").to_string()))]
    EjectVirtualDrive,
}

/// 是否为有效的文件路径（不包括通配符）
fn exist_file_parser(path: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(path);
    if !path.exists() {
        return Err(t!("value-parser.path-not-exist").to_string());
    };
    Ok(path)
}

/// 是否为有效的目录路径
fn exist_dir_parser(directory: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(directory);
    if !path.exists() {
        return Err(t!("value-parser.dir-not-exist").to_string());
    };
    if !path.is_dir() {
        return Err(t!("value-parser.not-dir").to_string());
    };
    Ok(path)
}

/// 是否为有效的路径（包括通配符）
fn exist_file_wildcard_parser(path: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(path);

    let file_name = path.file_name().unwrap().to_string_lossy();
    if file_name.contains('*') || file_name.contains('?') {
        return if path.parent().unwrap().exists() {
            Ok(path)
        } else {
            Err(t!("value-parser.path-not-exist").to_string())
        };
    }
    if !path.exists() {
        return Err(t!("value-parser.path-not-exist").to_string());
    }
    Ok(path)
}

/// 是否为系统路径
fn exist_system_parser(system_path: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(system_path);
    if !path.exists() {
        return Err(t!("value-parser.path-not-exist").to_string());
    };
    if !path.join(r"Windows\System32\ntoskrnl.exe").exists() {
        return Err(t!("value-parser.not-system-path").to_string());
    }
    Ok(path)
}

enum DriverClass {
    GUID_DEVCLASS_1394,
    GUID_DEVCLASS_1394DEBUG,
    GUID_DEVCLASS_61883,
    GUID_DEVCLASS_ADAPTER,
    GUID_DEVCLASS_APMSUPPORT,
    GUID_DEVCLASS_AVC,
    GUID_DEVCLASS_BATTERY,
    GUID_DEVCLASS_BIOMETRIC,
    GUID_DEVCLASS_BLUETOOTH,
    GUID_DEVCLASS_CAMERA,
    GUID_DEVCLASS_CDROM,
    GUID_DEVCLASS_COMPUTEACCELERATOR,
    GUID_DEVCLASS_COMPUTER,
    GUID_DEVCLASS_DECODER,
    GUID_DEVCLASS_DISKDRIVE,
    GUID_DEVCLASS_DISPLAY,
    GUID_DEVCLASS_DOT4,
    GUID_DEVCLASS_DOT4PRINT,
    GUID_DEVCLASS_EHSTORAGESILO,
    GUID_DEVCLASS_ENUM1394,
    GUID_DEVCLASS_EXTENSION,
    GUID_DEVCLASS_FDC,
    GUID_DEVCLASS_FIRMWARE,
    GUID_DEVCLASS_FLOPPYDISK,
    GUID_DEVCLASS_FSFILTER_ACTIVITYMONITOR,
    GUID_DEVCLASS_FSFILTER_ANTIVIRUS,
    GUID_DEVCLASS_FSFILTER_BOTTOM,
    GUID_DEVCLASS_FSFILTER_CFSMETADATASERVER,
    GUID_DEVCLASS_FSFILTER_COMPRESSION,
    GUID_DEVCLASS_FSFILTER_CONTENTSCREENER,
    GUID_DEVCLASS_FSFILTER_CONTINUOUSBACKUP,
    GUID_DEVCLASS_FSFILTER_COPYPROTECTION,
    GUID_DEVCLASS_FSFILTER_ENCRYPTION,
    GUID_DEVCLASS_FSFILTER_HSM,
    GUID_DEVCLASS_FSFILTER_INFRASTRUCTURE,
    GUID_DEVCLASS_FSFILTER_OPENFILEBACKUP,
    GUID_DEVCLASS_FSFILTER_PHYSICALQUOTAMANAGEMENT,
    GUID_DEVCLASS_FSFILTER_QUOTAMANAGEMENT,
    GUID_DEVCLASS_FSFILTER_REPLICATION,
    GUID_DEVCLASS_FSFILTER_SECURITYENHANCER,
    GUID_DEVCLASS_FSFILTER_SYSTEM,
    GUID_DEVCLASS_FSFILTER_SYSTEMRECOVERY,
    GUID_DEVCLASS_FSFILTER_TOP,
    GUID_DEVCLASS_FSFILTER_UNDELETE,
    GUID_DEVCLASS_FSFILTER_VIRTUALIZATION,
    GUID_DEVCLASS_GENERIC,
    GUID_DEVCLASS_GPS,
    GUID_DEVCLASS_HDC,
    GUID_DEVCLASS_HIDCLASS,
    GUID_DEVCLASS_HOLOGRAPHIC,
    GUID_DEVCLASS_IMAGE,
    GUID_DEVCLASS_INFINIBAND,
    GUID_DEVCLASS_INFRARED,
    GUID_DEVCLASS_KEYBOARD,
    GUID_DEVCLASS_LEGACYDRIVER,
    GUID_DEVCLASS_MEDIA,
    GUID_DEVCLASS_MEDIUM_CHANGER,
    GUID_DEVCLASS_MEMORY,
    GUID_DEVCLASS_MODEM,
    GUID_DEVCLASS_MONITOR,
    GUID_DEVCLASS_MOUSE,
    GUID_DEVCLASS_MTD,
    GUID_DEVCLASS_MULTIFUNCTION,
    GUID_DEVCLASS_MULTIPORTSERIAL,
    GUID_DEVCLASS_NET,
    GUID_DEVCLASS_NETCLIENT,
    GUID_DEVCLASS_NETDRIVER,
    GUID_DEVCLASS_NETSERVICE,
    GUID_DEVCLASS_NETTRANS,
    GUID_DEVCLASS_NETUIO,
    GUID_DEVCLASS_NODRIVER,
    GUID_DEVCLASS_PCMCIA,
    GUID_DEVCLASS_PNPPRINTERS,
    GUID_DEVCLASS_PORTS,
    GUID_DEVCLASS_PRIMITIVE,
    GUID_DEVCLASS_PRINTER,
    GUID_DEVCLASS_PRINTERUPGRADE,
    GUID_DEVCLASS_PRINTQUEUE,
    GUID_DEVCLASS_PROCESSOR,
    GUID_DEVCLASS_SBP2,
    GUID_DEVCLASS_SCMDISK,
    GUID_DEVCLASS_SCMVOLUME,
    GUID_DEVCLASS_SCSIADAPTER,
    GUID_DEVCLASS_SECURITYACCELERATOR,
    GUID_DEVCLASS_SENSOR,
    GUID_DEVCLASS_SIDESHOW,
    GUID_DEVCLASS_SMARTCARDREADER,
    GUID_DEVCLASS_SMRDISK,
    GUID_DEVCLASS_SMRVOLUME,
    GUID_DEVCLASS_SOFTWARECOMPONENT,
    GUID_DEVCLASS_SOUND,
    GUID_DEVCLASS_SYSTEM,
    GUID_DEVCLASS_TAPEDRIVE,
    GUID_DEVCLASS_UCM,
    GUID_DEVCLASS_UNKNOWN,
    GUID_DEVCLASS_USB,
    GUID_DEVCLASS_VOLUME,
    GUID_DEVCLASS_VOLUMESNAPSHOT,
    GUID_DEVCLASS_WCEUSBS,
    GUID_DEVCLASS_WPD,
}
