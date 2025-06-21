use crate::cli::validator::{isValidDirectory, isValidDriverClass, isValidPath, isValidPathIncludeWildcard, isValidSystemPath};
use crate::i18n::getLocaleText;
use clap::{Arg, ArgAction, ArgMatches, Command};

pub const HELP: &str = "help";
pub const SYSTEM_DRIVE: &str = "SystemDrive";
pub const DRIVE_PATH: &str = "DrivePath";
pub const INDEX_PATH: &str = "IndexPath";
pub const DRIVE_CLASS: &str = "DriveClass";
pub const ALL_DEVICE: &str = "AllDevice";
pub const MATCH_DEVICE: &str = "MatchDevice";
pub const PASSWORD: &str = "Password";
pub const OFFLINE_IMPORT: &str = "OfflineImport";
pub const EXTRACT_PATH: &str = "ExtractPath";
pub const EXPORT_PATH: &str = "ExportPath";
pub const DRIVER_NAME: &str = "DriverName";
pub const RENAME_DRIVER: &str = "RenameDriver";
pub const EJECTDRIVERCD: &str = "EjectDriverCD";
pub const PROGRAM_PATH: &str = "ProgramPath";
pub const SYSTEM_ROOT: &str = "SystemRoot";

pub fn cli() -> ArgMatches {
    Command::new(env!("CARGO_PKG_NAME"))
        // 基本配置
        .arg_required_else_help(true)
        .propagate_version(false)
        .version(env!("CARGO_PKG_VERSION"))
        // 模板
        .help_template(getLocaleText("template", None))
        .disable_version_flag(true)
        .arg(
            Arg::new("version")
                .short('V')
                .long("version")
                .help(getLocaleText("version-message", None))
                .action(ArgAction::Version)
        )
        .disable_help_flag(true)
        .disable_help_subcommand(true)
        .arg(
            Arg::new("help")
                .short('H')
                .long("help")
                .help(getLocaleText("help", None))
                .action(ArgAction::Help)
        )
        .subcommand(Command::new(HELP)
            .about(getLocaleText("help", None))
        )
        // Debug 模式
        .arg(
            Arg::new("debug")
                .short('D')
                .long("debug")
                .help(getLocaleText("on-debug", None)),
        )
        // 创建索引
        .subcommand(
            Command::new("create-index")
                .about(getLocaleText("create-index", None))
                .arg(
                    Arg::new(DRIVE_PATH)
                        .value_name(DRIVE_PATH)
                        .value_parser(isValidPath)
                        .required(true)
                        .index(1),
                )
                .arg(
                    Arg::new(INDEX_PATH)
                        .value_name(INDEX_PATH)
                        .index(2)
                        .help(getLocaleText("save-index-path", None)),
                )
                // 选项-指定压缩包密码
                .arg(
                    Arg::new(PASSWORD)
                        .short('p')
                        .long(PASSWORD)
                        .value_name(PASSWORD)
                        .help(getLocaleText("package-password", None)),
                ),
        )
        // 加载驱动
        .subcommand(
            Command::new("load-driver")
                .about(getLocaleText("load-driver", None))
                // 参数-驱动
                .arg(
                    Arg::new(DRIVE_PATH)
                        .value_name(DRIVE_PATH)
                        .value_parser(isValidPathIncludeWildcard)
                        .required(true)
                        .index(1)
                        .help(getLocaleText("package-path", None)),
                )
                // 参数-索引文件
                .arg(
                    Arg::new(INDEX_PATH)
                        .value_name(INDEX_PATH)
                        .index(2)
                        .help(getLocaleText("index-path", None)),
                )
                // 选项-指定压缩包密码
                .arg(
                    Arg::new(PASSWORD)
                        .short('p')
                        .long(PASSWORD)
                        .value_name(PASSWORD)
                        .help(getLocaleText("package-password", None)),
                )
                // 选项-匹配所有设备（包括已安装驱动设备）
                .arg(
                    Arg::new(ALL_DEVICE)
                        .short('a')
                        .long(ALL_DEVICE)
                        .help(getLocaleText("match-all-device", None)),
                )
                // 选项-驱动类别
                .arg(
                    Arg::new(DRIVE_CLASS)
                        .short('c')
                        .long(DRIVE_CLASS)
                        .value_name(DRIVE_CLASS)
                        .value_parser(isValidDriverClass)
                        .help(getLocaleText("driver-category", None)),
                )
                // 选项-仅解压不安装
                .arg(
                    Arg::new(EXTRACT_PATH)
                        .short('e')
                        .long(EXTRACT_PATH)
                        .value_name(EXTRACT_PATH)
                        .help(getLocaleText("only-unzip", None)),
                )
                // 选项-弹出免驱设备
                .arg(
                    Arg::new(EJECTDRIVERCD)
                        .short('j')
                        .long(EJECTDRIVERCD)
                        .help(getLocaleText("eject-driver-cd", None)),
                )
        )
        // 加载离线驱动
        .subcommand(
            Command::new("load-offline-driver")
                .about(getLocaleText("load-offline-driver", None))
                // 选项-系统盘
                .arg(
                    Arg::new(SYSTEM_DRIVE)
                        .short('s')
                        .long(SYSTEM_DRIVE)
                        .value_name(SYSTEM_DRIVE)
                        .value_parser(isValidSystemPath)
                )
                // 选项-匹配所有设备（包括已安装驱动设备）
                .arg(
                    Arg::new(ALL_DEVICE)
                        .short('a')
                        .long(ALL_DEVICE)
                        .help(getLocaleText("match-all-device", None)),
                )
                // 选项-驱动类别
                .arg(
                    Arg::new(DRIVE_CLASS)
                        .short('c')
                        .long(DRIVE_CLASS)
                        .value_name(DRIVE_CLASS)
                        .value_parser(isValidDriverClass)
                        .help(getLocaleText("driver-category", None)),
                )
        )
        // 导入驱动
        .subcommand(
            Command::new("import-driver")
                .about(getLocaleText("import-driver", None))
                // 参数-系统盘
                .arg(
                    Arg::new(SYSTEM_DRIVE)
                        .value_name(SYSTEM_DRIVE)
                        .value_parser(isValidSystemPath)
                        .required(true)
                        .index(1),
                )
                // 参数-驱动
                .arg(
                    Arg::new(DRIVE_PATH)
                        .value_name(DRIVE_PATH)
                        .value_parser(isValidPathIncludeWildcard)
                        .required(true)
                        .index(2)
                        .help(getLocaleText("package-path", None)),
                )
                // 选项-指定压缩包密码
                .arg(
                    Arg::new(PASSWORD)
                        .short('p')
                        .long(PASSWORD)
                        .value_name(PASSWORD)
                        .help(getLocaleText("package-password", None)),
                )
                // 选项-匹配所有设备（包括已安装驱动设备）
                .arg(
                    Arg::new(MATCH_DEVICE)
                        .short('m')
                        .long(MATCH_DEVICE)
                        .help(getLocaleText("match-device", None)),
                )
        )
        // 导出驱动
        .subcommand(
            Command::new("export-driver")
                .about(getLocaleText("export-driver", None))
                // 参数-系统盘
                .arg(
                    Arg::new(SYSTEM_DRIVE)
                        .value_name(SYSTEM_DRIVE)
                        .value_parser(isValidSystemPath)
                        .required(true)
                        .index(1),
                )
                // 参数-输出位置
                .arg(
                    Arg::new(EXPORT_PATH)
                        .value_name(EXPORT_PATH)
                        .required(true)
                        .index(2),
                )
                // 选项-驱动类别
                .arg(
                    Arg::new(DRIVE_CLASS)
                        .short('c')
                        .long(DRIVE_CLASS)
                        .value_name(DRIVE_CLASS)
                        .value_parser(isValidDriverClass)
                        .help(getLocaleText("driver-category", None)),
                )
                // 选项-驱动名称
                .arg(
                    Arg::new(DRIVER_NAME)
                        .short('n')
                        .long(DRIVER_NAME)
                        .value_name(DRIVER_NAME)
                        .help(getLocaleText("driver-name", None)),
                )
        )
        // 删除驱动
        .subcommand(
            Command::new("remove-driver")
                .about(getLocaleText("remove-driver", None))
                // 参数-系统盘
                .arg(
                    Arg::new(SYSTEM_DRIVE)
                        .value_name(SYSTEM_DRIVE)
                        .value_parser(isValidSystemPath)
                        .required(true)
                        .index(1),
                )
                // 参数-驱动
                .arg(
                    Arg::new(DRIVER_NAME)
                        .value_name(DRIVER_NAME)
                        .index(2)
                        .help(getLocaleText("driver-name", None)),
                )
                // 选项-驱动类别
                .arg(
                    Arg::new(DRIVE_CLASS)
                        .short('c')
                        .long(DRIVE_CLASS)
                        .value_name(DRIVE_CLASS)
                        .value_parser(isValidDriverClass)
                        .help(getLocaleText("driver-category", None)),
                )
        )
        // 整理驱动
        .subcommand(
            Command::new("classify-driver")
                .about(getLocaleText("classify-driver", None))
                // 参数-驱动位置
                .arg(
                    Arg::new(DRIVE_PATH)
                        .value_name(DRIVE_PATH)
                        .value_parser(isValidDirectory)
                        .required(true)
                        .index(1),
                )
                // 参数-输出位置
                .arg(
                    Arg::new(EXPORT_PATH)
                        .value_name(EXPORT_PATH)
                        .required(true)
                        .index(2),
                )
                // 选项-匹配所有设备（包括已安装驱动设备）
                .arg(
                    Arg::new(RENAME_DRIVER)
                        .short('r')
                        .long(RENAME_DRIVER)
                        .help(getLocaleText("rename_driver", None)),
                ),
        )
        // 创建驱动程序包
        .subcommand(
            Command::new("create-driver")
                .about(getLocaleText("create-driver", None))
                .arg(
                    Arg::new(DRIVE_PATH)
                        .value_name(DRIVE_PATH)
                        .value_parser(isValidPath)
                        .required(true)
                        .index(1)
                        .help(getLocaleText("package-path", None)),
                )
                .arg(
                    Arg::new(PROGRAM_PATH)
                        .value_name(PROGRAM_PATH)
                        .required(true)
                        .index(2)
                        .help(getLocaleText("driver-package-program-path", None)),
                )
        )
        // 扫描设备硬件更改
        .subcommand(
            Command::new("scan-devices")
                .about(getLocaleText("scan-devices", None))
        )
        .get_matches()
}
