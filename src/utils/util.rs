use crate::Asset;
use glob::MatchOptions;
use goblin::pe::PE;
use std::cmp::Ordering;
use std::env;
use std::error::Error;
use std::ffi::OsString;
use std::fs::{read, File, OpenOptions};
use std::io::Write;
use std::iter::repeat_with;
use std::path::{Path, PathBuf};

/// 写到文件
pub fn writeEmbedFile(filePath: &str, outFilePath: &Path) -> Result<(), Box<dyn Error>> {
    let file = Asset::get(filePath).unwrap();
    File::create(outFilePath)?.write_all(&file.data)?;
    Ok(())
}

/// 写日志
pub fn writeLogFile(logPath: &Path, content: &str) -> Result<(), Box<dyn Error>> {
    // 尝试创建文件
    if !logPath.exists() {
        File::create(logPath).expect("无法创建日志文件");
    }
    // 以追加模式打开文件
    let mut file = OpenOptions::new().append(true).open(logPath)?;
    file.write_all(format!("{}\r\n", content).as_bytes())?;
    Ok(())
}

/// 遍历目录及子目录下的所有指定文件
/// # 参数
/// 1. 目录路径
/// 2. 文件通配符 如 *.inf
pub fn getFileList(path: &Path, fileType: &str) -> Result<Vec<PathBuf>, Box<dyn Error>> {
    let srerch = glob::glob_with(
        &format!(r"{}\**\{}", path.to_str().unwrap(), fileType),
        MatchOptions {
            case_sensitive: false,
            require_literal_separator: false,
            require_literal_leading_dot: false,
        },
    )?;
    let fileList: Vec<PathBuf> = srerch
        .into_iter()
        .filter(|item| item.as_ref().unwrap().is_file())
        .map(|item| item.unwrap())
        .collect();
    Ok(fileList)
}

/// 是否为压缩包文件
pub fn isArchive(archivePath: &Path) -> bool {
    let extension = archivePath.extension().unwrap().to_str().unwrap_or("");
    let supportExtension = ["7z", "zip", "rar", "cab", "tar", "wim"];
    for item in supportExtension.iter() {
        if extension.to_lowercase() == *item.to_lowercase() {
            return true;
        }
    }
    false
}

/// 比较版本号大小
pub fn compareVersion(version1: &str, version2: &str) -> Result<Ordering, Box<dyn Error>> {
    let nums1: Vec<&str> = version1.split('.').collect();
    let nums2: Vec<&str> = version2.split('.').collect();
    let n1 = nums1.len();
    let n2 = nums2.len();

    // 比较版本
    for i in 0..std::cmp::max(n1, n2) {
        let i1 = match nums1.get(i) {
            Some(s) => s.parse::<u32>()?,
            None => 0,
        };
        let i2 = match nums2.get(i) {
            Some(s) => s.parse::<u32>()?,
            None => 0,
        };
        match i1.cmp(&i2) {
            Ordering::Equal => continue,
            non_eq => return Ok(non_eq),
        }
    }
    Ok(Ordering::Equal)
}

/// 生成临时文件名
/// # 参数
/// 1. 前缀
/// 2. 后缀
/// 3. 长度
pub fn getTmpName(prefix: &str, suffix: &str, rand_len: usize) -> OsString {
    let capacity = prefix
        .len()
        .saturating_add(suffix.len())
        .saturating_add(rand_len);
    let mut buf = OsString::with_capacity(capacity);
    buf.push(prefix);
    let mut char_buf = [0u8; 4];
    for c in repeat_with(fastrand::alphanumeric).take(rand_len) {
        buf.push(c.encode_utf8(&mut char_buf));
    }
    buf.push(suffix);
    buf
}

/// 提取指定字符串中全部%%形式的变量名
pub fn extract_vars(s: &str) -> Vec<String> {
    s.split('%')
        .enumerate()
        .filter_map(|(i, part)| {
            // 只保留奇数索引部分（两个%之间的内容）
            if i % 2 == 1 && !part.is_empty() {
                // 过滤合法字符（字母、数字、下划线）
                part.chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '_')
                    .then(|| part.to_string())
            } else {
                None
            }
        })
        .collect()
}

/// 是否为离线系统
pub fn isOfflineSystem(systemPath: &Path) -> Result<bool, Box<dyn Error>> {
    // 拼接 Windows 子目录
    let systemPath = PathBuf::from(systemPath).join("Windows");

    // 获取当前系统的 SystemRoot，如 C:\Windows
    let currentSystem = PathBuf::from(env::var("SystemRoot")?);

    let input_path = systemPath.canonicalize()?;
    let current_path = currentSystem.canonicalize()?;

    Ok(input_path != current_path)
}

/// # 获取系统架构
/// ## 参数
/// 1.系统目录
/// ## 返回值
/// - Ok(u16)：PE 文件 Machine 字段
///   - 0x014c → x86
///   - 0x8664 → x64
///   - 0xAA64 → ARM64
/// - Err：读取或解析失败
pub fn getArchCode(systemPath: &Path) -> Result<u16, Box<dyn Error>> {
    let krnl_path = systemPath.join("Windows").join("System32").join("ntoskrnl.exe");
    let bytes = read(&krnl_path)?;
    let pe = PE::parse(&bytes)?;

    let machine = pe.header.coff_header.machine;
    Ok(machine)
}

/// 查找离线系统盘
pub fn findOfflineSystemDrive() -> Vec<PathBuf> {
    let mut candidates = Vec::new();

    // 获取当前系统盘符
    let current_system_drive = env::var("SystemDrive").unwrap_or_else(|_| "C:".to_string());

    for letter in b'C'..=b'Z' {
        let drive = format!("{}:", letter as char);
        // 跳过当前系统盘
        if drive.eq_ignore_ascii_case(&current_system_drive) { continue; }
        let path = Path::new(&drive);
        if path.exists() && path.join("Windows").join("System32").join("ntoskrnl.exe").exists() {
            candidates.push(path.to_path_buf());
        }
    }
    candidates
}


// 增加字符串自定义方法
pub trait String_utils {
    fn get_string_left(&self, right: &str) -> Result<String, Box<dyn Error>>;
    fn get_string_center(&self, start: &str, end: &str) -> Result<String, Box<dyn Error>>;
    fn get_string_right(&self, left: &str) -> Result<String, Box<dyn Error>>;
}

impl String_utils for String {
    /// 取出字符串左边文本
    fn get_string_left(&self, right: &str) -> Result<String, Box<dyn Error>> {
        let endSize = self
            .find(right)
            .ok_or_else(|| "发生错误-查找结束位置失败".to_owned())?;
        Ok((self[..endSize]).to_string())
    }

    /// 取出字符串中间文本
    fn get_string_center(&self, start: &str, end: &str) -> Result<String, Box<dyn Error>> {
        let startSize = self
            .find(start)
            .ok_or_else(|| "发生错误-查找起始位置失败".to_owned())?;
        let endSize = startSize
            + self[startSize..]
            .find(end)
            .ok_or_else(|| "发生错误-查找结束位置失败".to_owned())?;
        Ok((self[startSize + start.len()..endSize]).to_string())
    }

    /// 取出字符串右边文本
    fn get_string_right(&self, left: &str) -> Result<String, Box<dyn Error>> {
        let startSize = self
            .find(left)
            .ok_or_else(|| "发生错误-查找左边位置失败".to_owned())?;
        Ok((self[startSize + left.len()..]).to_string())
    }
}
