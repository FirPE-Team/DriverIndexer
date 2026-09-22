use anyhow::{Result, anyhow};
use clap::{Parser, ValueEnum};
use rust_i18n::t;
use std::path::Path;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[clap(version)]
pub struct Cli {
    #[clap(subcommand)]
    pub command: Command,

    /// 设置程序语言
    #[clap(help = t!("options.lang").to_string())]
    #[clap(long, value_enum)]
    pub(crate) language: Option<Language>,

    /// 输出日志文件
    #[clap(long("log"), help(t!("options.log-path").to_string()))]
    pub log_path: Option<PathBuf>,

    /// 开启调试模式
    #[clap(long("debug"), help(t!("options.debug").to_string()))]
    pub debug: bool,
}

#[derive(Debug, Clone, ValueEnum)]
pub enum Language {
    /// English language
    En,
    /// Simplified Chinese
    ZhCn,
    /// Traditional Chinese
    ZhTw,
    /// Japanese language
    JaJp,
    /// Korean language
    KoKr,
}

#[derive(Parser, Debug)]
pub enum Command {
    /// 创建索引子命令
    #[clap(about(t!("commands.index").to_string()))]
    Index {
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

        /// 是否压缩索引文件
        #[clap(short, long)]
        #[clap(help(t!("options.compress-index").to_string()))]
        compress: bool,
    },

    /// 索引信息子命令
    #[clap(about(t!("commands.info").to_string()))]
    Info {
        /// 索引文件路径
        #[clap(help(t!("options.index-path").to_string()))]
        index_path: PathBuf,
    },

    /// 加载驱动子命令
    #[clap(about(t!("commands.install").to_string()))]
    Install {
        /// 驱动文件路径
        #[clap(value_parser = exist_file_wildcard_parser)]
        #[clap(help(t!("options.package-path").to_string()))]
        driver_path: PathBuf,

        /// 索引文件路径
        #[clap(short, long)]
        #[clap(help(t!("options.index-path").to_string()))]
        index_path: Option<PathBuf>,

        /// 是否跳过索引验证
        #[clap(short, long)]
        #[clap(help(t!("options.skip-verify").to_string()))]
        skip_verify: bool,

        /// 驱动密码
        #[clap(short, long)]
        #[clap(help(t!("options.package-password").to_string()))]
        password: Option<String>,

        /// 是否仅安装缺失的驱动
        #[clap(short = 'm', long)]
        #[clap(help(t!("options.missing-only").to_string()))]
        missing_only: bool,

        /// 驱动类别
        #[clap(short, long)]
        #[clap(help(t!("options.driver-category").to_string()))]
        class: Option<Vec<String>>,

        /// 排除的驱动类别
        #[clap(short = 'e', long)]
        #[clap(help(t!("options.exclude-category").to_string()))]
        exclude_class: Option<Vec<String>>,

        /// 是否仅解压驱动文件
        #[clap(short = 'x', long)]
        #[clap(help(t!("options.extract_to").to_string()))]
        extract_to: Option<PathBuf>,

        /// 是否强制加载驱动
        #[clap(short, long)]
        #[clap(help(t!("options.force").to_string()))]
        force: bool,
    },

    /// 加载离线驱动子命令
    #[clap(about(t!("commands.install-offline").to_string()))]
    InstallOffline {
        /// 系统盘符
        #[clap(value_parser = exist_system_parser)]
        #[clap(help(t!("options.system-drive").to_string()))]
        system_drive: Option<PathBuf>,

        /// 是否仅安装缺失的驱动
        #[clap(short = 'm', long)]
        #[clap(help(t!("options.missing-only").to_string()))]
        missing_only: bool,

        /// 驱动类别
        #[clap(short, long)]
        #[clap(help(t!("options.driver-category").to_string()))]
        class: Option<Vec<String>>,

        /// 排除的驱动类别
        #[clap(short = 'e', long)]
        #[clap(help(t!("options.exclude-category").to_string()))]
        exclude_class: Option<Vec<String>>,
    },

    /// 导入驱动子命令
    #[clap(about(t!("commands.import").to_string()))]
    Import {
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

        /// 是否匹配当前系统的设备
        #[clap(short, long)]
        #[clap(help(t!("options.match-device").to_string()))]
        match_device: bool,
    },

    /// 导出驱动子命令
    #[clap(about(t!("commands.export").to_string()))]
    Export {
        /// 系统盘符
        #[clap(index = 1, value_parser = exist_system_parser)]
        #[clap(help(t!("options.system-drive").to_string()))]
        system_drive: PathBuf,

        /// 导出路径
        #[clap(index = 2)]
        #[clap(help(t!("options.export-path").to_string()))]
        export_path: PathBuf,

        /// 包含系统自带的驱动
        #[clap(short = 's', long = "system")]
        #[clap(help(t!("options.include-system-driver").to_string()))]
        include_system: bool,

        /// 仅显示系统自带驱动
        #[clap(long = "system-only", conflicts_with = "include_system")]
        #[clap(help(t!("options.only-system-driver").to_string()))]
        system_only: bool,

        /// 指定驱动名称
        #[clap(short, long)]
        #[clap(help(t!("options.driver-name").to_string()))]
        inf: Option<String>,

        /// 指定驱动类别
        #[clap(short, long)]
        #[clap(help(t!("options.driver-category").to_string()))]
        class: Option<Vec<String>>,

        /// 排除的驱动类别
        #[clap(short = 'e', long)]
        #[clap(help(t!("options.exclude-category").to_string()))]
        exclude_class: Option<Vec<String>>,

        /// 指定驱动厂商
        #[clap(short, long)]
        #[clap(help(t!("options.driver-provider").to_string()))]
        provider: Option<Vec<String>>,
    },

    /// 删除驱动子命令
    #[clap(about(t!("commands.remove").to_string()))]
    #[clap(group(
        clap::ArgGroup::new("remove_filter")
            .required(true)
            .args(&["inf", "class", "provider", "all"])
            .multiple(true)
    ))]
    Remove {
        /// 系统盘符
        #[clap(index = 1, value_parser = exist_system_parser)]
        #[clap(help(t!("options.system-drive").to_string()))]
        system_drive: PathBuf,

        /// 包含系统自带的驱动
        #[clap(short = 's', long = "system")]
        #[clap(help(t!("options.include-system-driver").to_string()))]
        include_system: bool,

        /// 仅显示系统自带驱动
        #[clap(long = "system-only", conflicts_with = "include_system")]
        #[clap(help(t!("options.only-system-driver").to_string()))]
        system_only: bool,

        /// 指定驱动名称
        #[clap(short, long)]
        #[clap(help(t!("options.driver-name").to_string()))]
        inf: Option<String>,

        /// 指定驱动类别
        #[clap(short, long)]
        #[clap(help(t!("options.driver-category").to_string()))]
        class: Option<Vec<String>>,

        /// 指定驱动厂商
        #[clap(short, long)]
        #[clap(help(t!("options.driver-provider").to_string()))]
        provider: Option<Vec<String>>,

        /// 是否删除所有驱动
        #[clap(short, long)]
        #[clap(help(t!("options.remove-all-driver").to_string()))]
        all: bool,
    },

    /// 列举驱动子命令
    #[clap(about(t!("commands.list").to_string()))]
    List {
        /// 系统盘符
        #[clap(index = 1, value_parser = exist_system_parser)]
        #[clap(help(t!("options.system-drive").to_string()))]
        system_drive: PathBuf,

        /// 包含系统自带的驱动
        #[clap(short = 's', long = "system")]
        #[clap(help(t!("options.include-system-driver").to_string()))]
        include_system: bool,

        /// 仅显示系统自带驱动
        #[clap(long = "system-only", conflicts_with = "include_system")]
        #[clap(help(t!("options.only-system-driver").to_string()))]
        system_only: bool,

        /// 指定驱动类别
        #[clap(short, long)]
        #[clap(help(t!("options.driver-category").to_string()))]
        class: Option<Vec<String>>,

        /// 排除的驱动类别
        #[clap(short = 'e', long)]
        #[clap(help(t!("options.exclude-category").to_string()))]
        exclude_class: Option<Vec<String>>,

        /// 指定驱动厂商
        #[clap(short, long)]
        #[clap(help(t!("options.driver-provider").to_string()))]
        provider: Option<Vec<String>>,
    },

    /// 整理驱动子命令
    #[clap(about(t!("commands.organize").to_string()))]
    Organize {
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
    #[clap(about(t!("commands.pack").to_string()))]
    Pack {
        /// 驱动文件路径
        #[clap(index = 1, value_parser = exist_file_parser)]
        #[clap(help(t!("options.drive-path").to_string()))]
        drive_path: PathBuf,

        /// 驱动程序路径
        #[clap(index = 2)]
        #[clap(help(t!("options.driver-package-program-path").to_string()))]
        program_path: PathBuf,

        /// 驱动密码
        #[clap(short, long)]
        #[clap(help(t!("options.package-password").to_string()))]
        password: Option<String>,
    },

    /// 扫描设备硬件更改子命令
    #[clap(about(t!("commands.scan").to_string()))]
    Scan,

    /// 弹出免驱设备子命令
    #[clap(about(t!("commands.eject").to_string()))]
    Eject,

    /// 加密密码字符串，生成可用于 -p 参数的安全密文
    Encrypt {
        /// 要加密的明文密码
        #[clap(help(t!("options.encrypt-password").to_string()))]
        text: String,
    },
}

/// Resolve package paths and pair wildcard indexes by package stem.
pub(crate) fn resolve_package_index_pairs(
    package_path: &Path,
    index_path: Option<&Path>,
) -> Result<Vec<(PathBuf, Option<PathBuf>)>> {
    let is_wildcard = package_path
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.contains('*') || name.contains('?'));
    if !is_wildcard {
        return Ok(vec![(
            package_path.to_path_buf(),
            index_path.map(Path::to_path_buf),
        )]);
    }

    let parent = package_path
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .ok_or_else(|| anyhow!("package wildcard has no parent directory"))?;
    let pattern = package_path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| anyhow!("package wildcard is not valid UTF-8"))?;
    let mut packages = crate::utils::utils::get_file_list(parent, pattern)?;
    packages.sort_by(|left, right| {
        left.to_string_lossy()
            .to_ascii_lowercase()
            .cmp(&right.to_string_lossy().to_ascii_lowercase())
    });
    if packages.is_empty() {
        return Err(anyhow!("No driver package was found in this directory"));
    }

    let indexes = match index_path {
        None => Vec::new(),
        Some(path) => {
            let name = path
                .file_name()
                .and_then(|name| name.to_str())
                .ok_or_else(|| anyhow!("index wildcard is not valid UTF-8"))?;
            if name.contains('*') || name.contains('?') {
                let parent = path
                    .parent()
                    .filter(|path| !path.as_os_str().is_empty())
                    .ok_or_else(|| anyhow!("index wildcard has no parent directory"))?;
                let mut paths = crate::utils::utils::get_file_list(parent, name)?;
                paths.sort_by(|left, right| {
                    left.to_string_lossy()
                        .to_ascii_lowercase()
                        .cmp(&right.to_string_lossy().to_ascii_lowercase())
                });
                paths
            } else {
                vec![path.to_path_buf()]
            }
        }
    };

    let explicit_single = index_path
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| !name.contains('*') && !name.contains('?'))
        })
        .map(Path::to_path_buf);

    Ok(packages
        .into_iter()
        .map(|package| {
            let index = explicit_single.clone().or_else(|| {
                let stem = package.file_stem()?.to_string_lossy().to_ascii_lowercase();
                indexes.iter().find_map(|candidate| {
                    let candidate_stem = candidate
                        .file_stem()?
                        .to_string_lossy()
                        .to_ascii_lowercase();
                    (candidate_stem == stem).then(|| candidate.clone())
                })
            });
            (package, index)
        })
        .collect())
}

/// 是否为有效的文件路径（不包括通配符）
fn exist_file_parser(path: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(normalize_drive_root(path));
    if !path.exists() {
        return Err(t!("value-parser.path-not-exist").to_string());
    };
    Ok(path)
}

/// 是否为有效的目录路径
fn exist_dir_parser(directory: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(normalize_drive_root(directory));
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
    let path = PathBuf::from(normalize_drive_root(path));

    // 检查路径是否包含通配符
    if let Some(file_name) = path.file_name() {
        let file_name = file_name.to_string_lossy();
        if file_name.contains('*') || file_name.contains('?') {
            return if let Some(parent) = path.parent() {
                if !parent.exists() {
                    return Err(t!("value-parser.path-not-exist").to_string());
                }
                Ok(path)
            } else {
                Err(t!("value-parser.path-not-exist").to_string())
            };
        }
    }
    if !path.exists() {
        return Err(t!("value-parser.path-not-exist").to_string());
    }

    Ok(path)
}

/// 是否为系统路径
fn exist_system_parser(system_path: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(normalize_drive_root(system_path));
    if !path.exists() {
        return Err(t!("value-parser.path-not-exist").to_string());
    };
    if !path.join(r"Windows\System32\ntoskrnl.exe").exists() {
        return Err(t!("value-parser.not-system-path").to_string());
    }
    Ok(path)
}

/// 标准化系统盘符根路径，确保路径以反斜杠结尾。如果路径只有盘符（例如 “X:”），则在末尾添加反斜杠（例如 “X:\”）。
///
/// # 参数
///
/// - `s`：系统盘符根路径（例如 “X:” 或 “X:\”）
///
/// # 返回值
///
/// - `String`：标准化后的系统盘符根路径（例如 “X:\”）
fn normalize_drive_root(s: &str) -> String {
    if s.len() == 2 && s.as_bytes()[1] == b':' && s.as_bytes()[0].is_ascii_alphabetic() {
        format!("{}\\", s)
    } else {
        s.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::resolve_package_index_pairs;
    use std::fs;

    #[test]
    fn pairs_wildcard_indexes_by_package_stem() {
        let root =
            std::env::temp_dir().join(format!("driver-cli-pair-test-{}", std::process::id()));
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("b.7z"), b"b").unwrap();
        fs::write(root.join("a.7z"), b"a").unwrap();
        fs::write(root.join("a.index"), b"a").unwrap();
        fs::write(root.join("unrelated.index"), b"x").unwrap();

        let pairs =
            resolve_package_index_pairs(&root.join("*.7z"), Some(&root.join("*.index"))).unwrap();
        assert_eq!(pairs.len(), 2);
        assert_eq!(pairs[0].0.file_name().unwrap(), "a.7z");
        assert_eq!(pairs[0].1.as_ref().unwrap().file_name().unwrap(), "a.index");
        assert_eq!(pairs[1].0.file_name().unwrap(), "b.7z");
        assert!(pairs[1].1.is_none());

        fs::remove_dir_all(root).unwrap();
    }
}
