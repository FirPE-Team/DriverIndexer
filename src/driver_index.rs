use crate::utils::utils::{compare_version, extract_vars, format_bytes, String_utils};
use anyhow::{anyhow, Context, Result};
use bincode::{Decode, Encode};
use chrono::NaiveDate;
use encoding::{label, DecoderTrap};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::HashSet;
use std::fs::File;
use std::io::Read;
use std::path::Path;

/// 驱动索引
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Encode, Decode)]
pub struct DriverIndex {
    /// 索引文件大小
    pub size: u64,

    /// 索引数据
    pub drivers: Vec<InfInfo>,
}

/// INF驱动信息
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Encode, Decode)]
pub struct InfInfo {
    /// 驱动路径
    pub path: String,

    /// 驱动类别
    pub class: String,

    /// 驱动位宽
    pub arch: Vec<DriverArch>,

    /// 驱动日期
    pub date: String,

    /// 驱动版本
    pub version: String,

    /// 硬件ID列表
    pub hwid: Vec<String>,

    /// 兼容ID列表
    pub cid: Vec<String>,
}

/// 系统架构
/// https://learn.microsoft.com/zh-cn/windows-hardware/drivers/install/creating-inf-files-for-multiple-platforms-and-operating-systems
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Encode, Decode)]
pub enum DriverArch {
    /// NT x86架构
    NTx86,

    /// NT amd64架构
    NTamd64,

    /// NT ia64架构
    NTia64,

    /// NT arm架构
    NTarm,

    /// NT arm64架构
    NTarm64,

    /// NT 架构（未指定）
    Nt,
}

impl DriverIndex {
    /// 创建新的索引文件
    /// # 参数
    /// - `size`: 索引文件大小
    /// - `info`: INF驱动信息列表
    ///
    /// # 返回值
    /// - `DriverIndex`: 新的驱动索引
    pub fn new(size: u64, info: Vec<InfInfo>) -> Self {
        Self {
            size,
            drivers: info,
        }
    }

    /// 获取驱动索引信息
    ///
    /// # 返回值
    /// - `String`: 驱动索引信息字符串
    pub fn get_driver_index_info(&self) -> String {
        let mut result = String::new();

        let label_w = 15;
        let total_w = label_w + 10;
        result.push_str("Driver Index Info:\n");
        result.push_str(&format!("{:-^total_w$}\n", "-", total_w = total_w));

        // 驱动大小
        result.push_str(&format!(
            "{:<width$} {}\n",
            "Driver Size:",
            format_bytes(self.size),
            width = label_w
        ));

        // 驱动数量
        result.push_str(&format!(
            "{:<width$} {:?}\n",
            "Driver Count:",
            self.drivers.len(),
            width = label_w
        ));

        // 驱动类别
        result.push_str(&format!(
            "{:<width$} {:?}\n",
            "Driver Classes:",
            self.drivers
                .iter()
                .map(|x| &x.class)
                .collect::<HashSet<_>>()
                .into_iter()
                .collect::<Vec<_>>(),
            width = label_w
        ));

        // 统计驱动硬件ID、兼容ID数量
        let total_hwid_count = self.drivers.iter().map(|x| x.hwid.len()).sum::<usize>();
        let total_cid_count = self.drivers.iter().map(|x| x.cid.len()).sum::<usize>();
        result.push_str(&format!(
            "{:<width$} {} ({} Hardware ID, {} Compatible ID)\n",
            "Total HWID Count:",
            total_hwid_count + total_cid_count,
            total_hwid_count,
            total_cid_count,
            width = label_w
        ));

        result
    }

    /// 解析索引数据
    ///
    /// # 参数
    /// - `index_path`: 索引文件路径
    ///
    /// # 返回值
    /// - `Ok(Vec<InfInfo>)`: 解析后的INF驱动信息列表
    pub fn from_json(path: &Path) -> Result<DriverIndex> {
        let mut config_file =
            File::open(path).with_context(|| format!("open file {:?} failed", path))?;
        let mut content = String::new();
        config_file
            .read_to_string(&mut content)
            .with_context(|| format!("read index file {:?}", path))?;
        serde_json::from_str(&content).with_context(|| format!("parse index file {:?}", path))
    }

    /// 保存INF数据（通过JSON）
    ///
    /// # 参数
    /// - `data`: INF驱动信息列表
    /// - `path`: 索引文件保存路径
    ///
    /// # 返回值
    /// - `Ok(())`: 成功
    /// - `Err(Error)`: 失败（包含错误信息）
    pub fn to_json(&self) -> serde_json::Result<String> {
        serde_json::to_string(&self)
    }
}

impl DriverArch {
    pub fn display(self) -> &'static str {
        match self {
            DriverArch::NTx86 => "NTx86",
            DriverArch::NTamd64 => "NTamd64",
            DriverArch::NTia64 => "NTia64",
            DriverArch::NTarm => "NTarm",
            DriverArch::NTarm64 => "NTarm64",
            DriverArch::Nt => "Nt",
        }
    }
}

impl InfInfo {
    /// 解析INF文件（通过按行读取）
    ///
    /// # 参数
    /// - `base_path`: inf 基本路径（父路径）
    /// - `inf_file`: inf 文件路径
    ///
    /// # 返回值
    /// - `Ok(InfInfo)`: 解析后的INF驱动信息
    pub fn parse_from_inf(base_path: &Path, inf_file: &Path) -> Result<InfInfo> {
        // 打开INF文件
        let mut file = File::open(inf_file)
            .with_context(|| format!("Open inf file Failed: {:?}", inf_file))?;

        // 读取INF文件
        let mut buffer: Vec<u8> = Vec::new();
        file.read_to_end(&mut buffer)
            .with_context(|| format!("Read inf file Failed: {:?}", inf_file))?;

        // 自动识别编码并以UTF-8读取
        let result = chardet::detect(&buffer);
        let coder = label::encoding_from_whatwg_label(chardet::charset2encoding(&result.0))
            .with_context(|| "Detect INF file encoding failed".to_string())?;
        let inf_content = match coder.decode(&buffer, DecoderTrap::Ignore) {
            Ok(content) => content,
            Err(e) => {
                return Err(anyhow!("Decode inf file failed: {}", e));
            }
        };

        // 去除INF内的所有 空格 与 tab符
        let inf_content = inf_content.replace(" ", "").replace("	", "");

        let mut class = String::new();
        let mut date = String::new();
        let mut version = String::new();
        let mut arch: Vec<DriverArch> = Vec::new();
        let mut hwid: Vec<String> = Vec::new();
        let mut cid: Vec<String> = Vec::new();

        // 按行读取
        for line in inf_content.lines() {
            // 跳过空行、注释行
            if line.is_empty() || line.starts_with(";") {
                continue;
            }

            // 去除行尾注释
            let line = line.split(';').next().unwrap_or(line).trim();

            // 变量替换处理
            let line = extract_vars(line)
                .iter()
                .fold(line.to_string(), |acc, ver| {
                    inf_content
                        .get_string_center(&format!("{ver}="), "\r\n")
                        .map(|v| acc.replace(&format!("%{ver}%"), v.trim_matches('"')))
                        .unwrap_or(acc)
                });

            // 转换为小写
            let lower_line = line.to_lowercase();

            // 驱动类别
            if let Some(c) = lower_line.strip_prefix("class=") {
                // 首字母大写
                class = c[0..1].to_uppercase() + &c[1..];
            }

            // 驱动版本、日期
            if let Some(date_and_version) = lower_line.strip_prefix("driverver=") {
                let (mut d, v) = date_and_version
                    .split_once(',')
                    .map(|(d, v)| (d.trim(), v.trim()))
                    .unwrap_or((date_and_version, ""));

                // 去掉前导非数字（例如 "Thu03/14/2002"、"Thu 03/14/2002"）
                if let Some(pos) = d.find(|c: char| c.is_ascii_digit()) {
                    d = &d[pos..];
                }
                date = match NaiveDate::parse_from_str(d, "%m/%d/%Y") {
                    Ok(dt) => dt,
                    Err(_) => NaiveDate::parse_from_str(d, "%Y/%m/%d")
                        .with_context(|| format!("parse date failed: {}", d))?,
                }
                .format("%Y-%m-%d")
                .to_string();

                version = v.to_string();
            }

            // 驱动平台
            if lower_line.contains(".ntx86") && !arch.contains(&DriverArch::NTx86) {
                arch.push(DriverArch::NTx86);
            }
            if lower_line.contains(".ntamd64") && !arch.contains(&DriverArch::NTamd64) {
                arch.push(DriverArch::NTamd64);
            }
            if lower_line.contains(".ntia64") && !arch.contains(&DriverArch::NTia64) {
                arch.push(DriverArch::NTia64);
            }
            if lower_line.contains(".ntarm") && !arch.contains(&DriverArch::NTarm) {
                arch.push(DriverArch::NTarm);
            }
            if lower_line.contains(".ntarm64") && !arch.contains(&DriverArch::NTarm64) {
                arch.push(DriverArch::NTarm64);
            }
            if (lower_line.contains(".nt")
                && !lower_line.contains(".ntx86")
                && !lower_line.contains(".ntamd64")
                && !lower_line.contains(".ntia64")
                && !lower_line.contains(".ntarm")
                && !lower_line.contains(".ntarm64"))
                && !arch.contains(&DriverArch::Nt)
            {
                arch.push(DriverArch::Nt);
            }

            // 获取硬件ID（如果存在等于号并且逗号分隔则获取逗号之后的部分）
            if let Some(equal_pos) = line.find('=')
                && let Some(comma_pos) = line[equal_pos..].find(',')
            {
                // 获取逗号之后的部分
                let potential_id = &line[(equal_pos + comma_pos + 1)..].trim();

                // 排除关键字
                let exclude_keywords = [
                    "SYSWOW32",
                    "SYSWOW64",
                    "PROGRAMDATA",
                    "\\X86",
                    "\\X64",
                    "\\AMD64",
                    "\\I386",
                ];
                if exclude_keywords
                    .iter()
                    .any(|k| potential_id.to_uppercase().contains(k))
                {
                    continue;
                }

                // 逗号分隔硬件ID、兼容ID
                let mut first_id = true;
                for id in potential_id.split(",") {
                    // 检查硬件ID特征（必须包含反斜杠 或 开头为星号）
                    if id == "\\" || (!id.contains('\\') && !id.starts_with('*')) {
                        continue;
                    }

                    // 检查是否符合硬件ID格式
                    if !id.chars().all(|c| {
                        c.is_ascii_alphanumeric()
                            || c == '\\'
                            || c == '&'
                            || c == '_'
                            || c == '.'
                            || c == '-'
                            || c == '*'
                            || c == ':'
                            || c == '{'
                            || c == '}'
                    }) {
                        continue;
                    }

                    if first_id {
                        if !hwid.contains(&id.to_uppercase()) {
                            hwid.push(id.to_uppercase());
                        }
                        first_id = false;
                    } else {
                        if !cid.contains(&id.to_uppercase()) {
                            cid.push(id.to_uppercase());
                        }
                    }
                }
            }
        }

        // 转换为相对路径
        let inf_path = inf_file
            .strip_prefix(base_path)
            .with_context(|| "Strip inf path prefix failed")?;

        Ok(InfInfo {
            path: inf_path.to_string_lossy().to_string(),
            class,
            arch,
            date,
            version,
            hwid,
            cid,
        })
    }

    /// 对 INF 信息列表进行排序
    ///
    /// # 参数
    ///
    /// - `infos` - INF 信息列表的可变引用
    ///
    /// # 排序规则
    ///
    /// 1. class (case-insensitive)
    /// 2. version (希望高版本在前)
    /// 3. date (YYYY-MM-DD 格式可直接比较)
    /// 4. inf 文件名 (升序)
    pub fn sort_inf_list(infos: &mut [InfInfo]) {
        infos.sort_by(|a, b| {
            // 1. class (case-insensitive)
            let ca = a.class.to_lowercase();
            let cb = b.class.to_lowercase();
            match ca.cmp(&cb) {
                Ordering::Equal => {
                    // 2. version (希望高版本在前) -> 使用 compare_version 并倒序
                    match compare_version(&a.version, &b.version) {
                        Ok(ord) => {
                            if ord == Ordering::Equal {
                                // 3. date (YYYY-MM-DD 格式可直接比较)，降序
                                match a.date.cmp(&b.date) {
                                    Ordering::Equal => {
                                        // 4. 最后按 inf 文件名升序稳定决定
                                        a.path.to_lowercase().cmp(&b.path.to_lowercase())
                                    }
                                    other => other.reverse(), // 降序
                                }
                            } else {
                                ord.reverse() // 反转以得到降序
                            }
                        }
                        Err(_) => {
                            // 如果 compare_version 失败，退回到字符串比较（降序）
                            match b.version.cmp(&a.version) {
                                Ordering::Equal => {
                                    a.path.to_lowercase().cmp(&b.path.to_lowercase())
                                }
                                ord => ord,
                            }
                        }
                    }
                }
                ord => ord, // class 升序
            }
        });
    }
}
