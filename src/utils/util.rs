use std::cmp::Ordering;
use std::error::Error;
use std::ffi::OsString;
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::iter::repeat_with;
use std::path::{Path, PathBuf};
use crate::Asset;
use glob::MatchOptions;

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
