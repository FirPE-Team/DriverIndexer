use clap::{crate_version, App, AppSettings, Arg, ArgMatches, SubCommand};

use crate::cli::validator::{isValidDirectory, isValidDriverClass, isValidPath, isValidPathIncludeWildcard, isValidSystemPath};
use crate::i18n::getLocaleText;

pub const HELP: &str = "help";
pub const SYSTEM_DRIVE: &str = "SystemDrive";
pub const DRIVE_PATH: &str = "DrivePath";
pub const INDEX_PATH: &str = "IndexPath";
pub const DRIVE_CLASS: &str = "DriveClass";
pub const ALL_DEVICE: &str = "AllDevice";
pub const MATCH_DEVICE: &str = "MatchDevice";
pub const PASSWORD: &str = "Password";
pub const OFFLINE_IMPORT: &str = "OfflineImport";
pub const EXTRACT_DRIVER: &str = "ExtractDriver";
pub const PROGRAM_PATH: &str = "ProgramPath";
pub const SYSTEM_ROOT: &str = "SystemRoot";

pub fn cli<'a>() -> ArgMatches<'a> {
    App::new(crate_name!())
        // 基本配置
        .setting(AppSettings::ArgRequiredElseHelp)
        .setting(AppSettings::VersionlessSubcommands)
        .global_setting(AppSettings::UnifiedHelpMessage)
        .version(crate_version!())
        // 模板
        .template(&*getLocaleText("template", None))
        .help_message(&*Box::leak(
            getLocaleText("help-message", None).into_boxed_str(),
        ))
        .version_message(&*Box::leak(
            getLocaleText("version-message", None).into_boxed_str(),
        ))
        .help_short("H")
        .subcommand(SubCommand::with_name(HELP)
            .help_short("H")
            .about(&*getLocaleText("help", None))
        )

        // Debug 模式
        .arg(
            Arg::with_name("debug")
                .short("D")
                .long("debug")
                .help(&getLocaleText("on-debug", None)),
        )
        // 创建索引
        .subcommand(
            SubCommand::with_name("create-index")
                .about(&*getLocaleText("create-index", None))
                .help_short("H")
                .arg(
                    Arg::with_name(DRIVE_PATH)
                        .value_name(DRIVE_PATH)
                        .validator(isValidPath)
                        .required(true)
                        .index(1),
                )
                .arg(
                    Arg::with_name(INDEX_PATH)
                        .value_name(INDEX_PATH)
                        .index(2)
                        .help(&getLocaleText("save-index-path", None)),
                )
                // 选项-指定压缩包密码
                .arg(
                    Arg::with_name(PASSWORD)
                        .short("p")
                        .long(PASSWORD)
                        .value_name(PASSWORD)
                        .help(&getLocaleText("package-password", None)),
                ),
        )
        // 加载驱动
        .subcommand(
            SubCommand::with_name("load-driver")
                .about(&*getLocaleText("load-driver", None))
                .help_short("H")
                // 参数-驱动
                .arg(
                    Arg::with_name(DRIVE_PATH)
                        .value_name(DRIVE_PATH)
                        .validator(isValidPathIncludeWildcard)
                        .required(true)
                        .index(1)
                        .help(&getLocaleText("package-path", None)),
                )
                // 参数-索引文件
                .arg(
                    Arg::with_name(INDEX_PATH)
                        .value_name(INDEX_PATH)
                        .index(2)
                        .help(&getLocaleText("index-path", None)),
                )
                // 选项-指定压缩包密码
                .arg(
                    Arg::with_name(PASSWORD)
                        .short("p")
                        .long(PASSWORD)
                        .value_name(PASSWORD)
                        .help(&getLocaleText("package-password", None)),
                )
                // 选项-匹配所有设备（包括已安装驱动设备）
                .arg(
                    Arg::with_name(ALL_DEVICE)
                        .short("a")
                        .long(ALL_DEVICE)
                        .help(&getLocaleText("match-all-device", None)),
                )
                // 选项-驱动类别
                .arg(
                    Arg::with_name(DRIVE_CLASS)
                        .short("c")
                        .long(DRIVE_CLASS)
                        .value_name(DRIVE_CLASS)
                        .validator(isValidDriverClass)
                        .help(&getLocaleText("driver-category", None)),
                )
                // 选项-仅解压不安装
                .arg(
                    Arg::with_name(EXTRACT_DRIVER)
                        .short("e")
                        .long(EXTRACT_DRIVER)
                        .value_name(EXTRACT_DRIVER)
                        .help(&getLocaleText("only-unzip", None)),
                )
        )
        // 导入驱动
        .subcommand(
            SubCommand::with_name("import-driver")
                .about(&*getLocaleText("import-driver", None))
                .help_short("H")
                // 参数-系统盘
                .arg(
                    Arg::with_name(SYSTEM_DRIVE)
                        .value_name(SYSTEM_DRIVE)
                        .validator(isValidSystemPath)
                        .required(true)
                        .index(1),
                )
                // 参数-驱动
                .arg(
                    Arg::with_name(DRIVE_PATH)
                        .value_name(DRIVE_PATH)
                        .validator(isValidPathIncludeWildcard)
                        .required(true)
                        .index(2)
                        .help(&getLocaleText("package-path", None)),
                )
                // 选项-指定压缩包密码
                .arg(
                    Arg::with_name(PASSWORD)
                        .short("p")
                        .long(PASSWORD)
                        .value_name(PASSWORD)
                        .help(&getLocaleText("package-password", None)),
                )
                // 选项-匹配所有设备（包括已安装驱动设备）
                .arg(
                    Arg::with_name(MATCH_DEVICE)
                        .short("m")
                        .long(MATCH_DEVICE)
                        .help(&getLocaleText("match-device", None)),
                )
        )
        // 整理驱动
        .subcommand(
            SubCommand::with_name("classify-driver")
                .about(&*getLocaleText("classify-driver", None))
                .help_short("H")
                .arg(
                    Arg::with_name(DRIVE_PATH)
                        .value_name(DRIVE_PATH)
                        .validator(isValidDirectory)
                        .required(true)
                        .index(1),
                ),
        )
        // 创建驱动程序包
        .subcommand(
            SubCommand::with_name("create-driver")
                .about(&*getLocaleText("create-driver", None))
                .help_short("H")
                .arg(
                    Arg::with_name(DRIVE_PATH)
                        .value_name(DRIVE_PATH)
                        .validator(isValidPath)
                        .required(true)
                        .index(1)
                        .help(&getLocaleText("package-path", None)),
                )
                .arg(
                    Arg::with_name(PROGRAM_PATH)
                        .value_name(PROGRAM_PATH)
                        .required(true)
                        .index(2)
                        .help(&getLocaleText("driver-package-program-path", None)),
                )
        )
        .get_matches()
}
