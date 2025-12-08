use crate::utils::setupapi::SetupAPI;
use crate::utils::utils::{
    check_catalog_signature, compare_version, format_bytes, is_whql_signature,
};
use anyhow::{Context, Result};
use bincode::{config, Decode, Encode};
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::HashSet;
use std::fs::File;
use std::io::Read;
use std::path::Path;

/// 驱动索引
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Encode, Decode)]
pub struct DriverIndex {
    /// 索引文件大小（字节）
    pub size: u64,
    /// 索引文件修改时间戳（Unix 时间戳）
    pub timestamp: u64,
    /// 索引数据（INF驱动信息列表）
    pub drivers: Vec<InfInfo>,
}

/// INF驱动信息
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Encode, Decode)]
pub struct InfInfo {
    /// 驱动路径
    pub path: String,

    /// 驱动类别（例如: "net", "storage", "audio" 等）
    pub class: String,

    /// 驱动日期（格式: "YYYY-MM-DD"）
    pub date: String,

    /// 驱动版本（例如: "1.0.0.0"）
    #[serde(rename = "ver")]
    pub version: String,

    /// 驱动签名状态（例如: "None", "Signed", "Whql" 等）
    #[serde(rename = "sign")]
    pub signature: SignatureStatus,

    /// 硬件ID列表
    pub hardware: Vec<HardwareEntry>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Encode, Decode)]
pub struct HardwareEntry {
    /// 硬件描述
    pub desc: String,

    /// 驱动位宽
    pub arch: DriverArch,

    /// 驱动支持最低系统版本
    /// 例如: "10.0", "6.1", 或者 "" (表示通用/未指定)
    #[serde(rename = "os")]
    pub min_os_version: String,

    /// 硬件ID
    #[serde(rename = "hwid")]
    pub hardware_id: String,

    /// 兼容ID列表
    #[serde(rename = "cids")]
    pub compatible_ids: Vec<String>,
}

/// 驱动签名状态
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Encode, Decode,
)]
pub enum SignatureStatus {
    /// 未签名或签名无效（不可用）
    None,
    /// 普通商业签名 (虽然受信任，但优先级较低)
    Signed,
    /// 微软 WHQL 签名 (最高优先级)
    Whql,
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
    pub fn new(size: u64, timestamp: u64, drivers: Vec<InfInfo>) -> Self {
        Self {
            size,
            timestamp,
            drivers,
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
        let hardware_id_count = self.drivers.iter().map(|x| x.hardware.len()).sum::<usize>();
        let compatible_id_count = self
            .drivers
            .iter()
            .map(|x| {
                x.hardware
                    .iter()
                    .map(|y| y.compatible_ids.len())
                    .sum::<usize>()
            })
            .sum::<usize>();
        result.push_str(&format!(
            "{:<width$} {} ({} Hardware ID, {} Compatible ID)\n",
            "Total Hardware ID Count:",
            hardware_id_count + compatible_id_count,
            hardware_id_count,
            compatible_id_count,
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

    /// 将索引数据转换为JSON字符串
    ///
    /// # 返回值
    /// - `Ok(())`: 成功
    /// - `Err(Error)`: 失败（包含错误信息）
    pub fn to_json(&self) -> serde_json::Result<String> {
        serde_json::to_string(&self)
    }

    /// 将索引数据转换为Bincode编码的字节向量
    ///
    /// # 返回值
    /// - `Ok(())`: 成功
    /// - `Err(Error)`: 失败（包含错误信息）
    pub fn to_binary(&self) -> Result<Vec<u8>> {
        let config = config::standard();
        Ok(bincode::encode_to_vec(self, config)?)
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
    /// 解析INF文件
    ///
    /// # 参数
    /// - `base_path`: inf 基本路径（父路径）
    /// - `inf_file`: inf 文件路径
    ///
    /// # 返回值
    /// - `Ok(InfInfo)`: 解析后的INF驱动信息
    pub fn parse_inf(base_path: &Path, inf_file: &Path) -> Result<InfInfo> {
        let handle_inf = SetupAPI::open_inf_file(inf_file)
            .with_context(|| "Open inf file failed".to_string())?;

        // 查找Class字段
        let class_context = SetupAPI::find_first_line(handle_inf, "Version", Some("Class"))
            .with_context(|| "Find class line failed".to_string())?;
        let class = SetupAPI::get_string_field(&class_context, 1)
            .with_context(|| "Get class failed".to_string())?;

        // 查找DriverVer段
        let driver_ver_context =
            SetupAPI::find_first_line(handle_inf, "Version", Some("DriverVer"))
                .with_context(|| "Find version line failed".to_string())?;

        // 解析Date字段
        let mut date = SetupAPI::get_string_field(&driver_ver_context, 1)
            .with_context(|| "Get date failed".to_string())?;
        // 去掉前导非数字（例如 "Thu03/14/2002"、"Thu 03/14/2002"）
        if let Some(pos) = date.find(|c: char| c.is_ascii_digit()) {
            date = date[pos..].to_string();
        }
        // 格式化日期格式为YYYY-MM-DD
        date = match NaiveDate::parse_from_str(&date, "%m/%d/%Y") {
            Ok(dt) => dt,
            Err(_) => NaiveDate::parse_from_str(&date, "%Y/%m/%d")
                .with_context(|| format!("Format date failed: {}", date))?,
        }
        .format("%Y-%m-%d")
        .to_string();

        // 解析Version字段
        let version = SetupAPI::get_string_field(&driver_ver_context, 2)
            .with_context(|| "Get version failed".to_string())?;

        // 查找CatalogFile字段
        const SEARCH_KEYS: [&str; 5] = [
            "CatalogFile.NTamd64", // 针对 x64
            "CatalogFile.NTx86",   // 针对 x86
            "CatalogFile.NTarm64", // 针对 ARM64
            "CatalogFile.NT",      // 针对 NT 核心通用
            "CatalogFile",         // 最古老/最通用
        ];
        let mut signature = SignatureStatus::None;
        for key in SEARCH_KEYS {
            // 尝试获取该 Key 对应的字符串值
            if let Ok(filename_context) =
                SetupAPI::find_first_line(handle_inf, "Version", Some(key))
            {
                if let Ok(filename) = SetupAPI::get_string_field(&filename_context, 1) {
                    let catalog_file = inf_file.parent().unwrap().join(filename);
                    if catalog_file.exists() {
                        signature = if check_catalog_signature(&catalog_file) {
                            // 检查是否包含 WHQL 签名
                            if is_whql_signature(&catalog_file) {
                                SignatureStatus::Whql
                            } else {
                                SignatureStatus::Signed
                            }
                        } else {
                            SignatureStatus::None
                        };
                    }
                    break;
                }
            }
        }

        // 用于存储推导出的所有目标节名 (例如 [Realtek], [Realtek.NTamd64])
        let mut candidate_sections: Vec<String> = Vec::new();

        // 遍历 [Manufacturer] 节
        let mut manufacturer_context = SetupAPI::find_first_line(handle_inf, "Manufacturer", None)
            .with_context(|| "Find manufacturer line failed".to_string())?;
        loop {
            let field_count = SetupAPI::get_field_count(&manufacturer_context);
            if let Ok(base_name) = SetupAPI::get_string_field(&manufacturer_context, 1) {
                // 添加基础节 (例如 "Realtek")
                candidate_sections.push(base_name.clone());

                // 步骤 2: 遍历后续字段 (从 Field 2 开始) 进行拼接
                // 如果 field_count 是 1，这里范围是 2..=1 (为空)，循环不会执行，逻辑正确兼容
                if field_count >= 2 {
                    for i in 2..=field_count {
                        // 获取后缀 (例如 "NTamd64")
                        if let Ok(suffix) = SetupAPI::get_string_field(&manufacturer_context, i) {
                            // 添加组合节 (例如 "Realtek.NTamd64")
                            let full_section_name = format!("{}.{}", base_name, suffix);
                            candidate_sections.push(full_section_name);
                        }
                    }
                }
            }

            match SetupAPI::find_next_line(&mut manufacturer_context) {
                // 更新 context 继续循环
                Ok(context) => manufacturer_context = context,
                // 没有下一行，退出循环
                Err(_) => break,
            };
        }

        // 遍历所有硬件ID
        let mut hardware_entries: Vec<HardwareEntry> = Vec::new();
        for section_name in candidate_sections {
            // 尝试查找该节的第一行
            let mut model_context = match SetupAPI::find_first_line(handle_inf, &section_name, None)
            {
                Ok(ctx) => ctx,
                // 节不存在，跳过（这很正常，因为有些推导出来的后缀组合可能在 INF 里没写）
                Err(_) => continue,
            };

            // 解析系统架构
            let (arch, os_version) = parse_section_metadata(&section_name);

            // 节存在，开始遍历该节的每一行
            loop {
                // Field 0: 设备名称 (Name)
                let name = SetupAPI::get_string_field(&model_context, 0)
                    .unwrap_or_else(|_| "Unknown Device".to_string());

                // Field 1: Install Section Name (如 "DriverInstall_Section")，通常不需要索引
                // Field 2: Main Hardware ID (如 "PCI\VEN_10EC&DEV_8168&SUBSYS_00008168&REV_00")
                if let Ok(hw_id) = SetupAPI::get_string_field(&model_context, 2) {
                    let hardware_id = hw_id.to_uppercase();

                    // 获取Compatible IDs(Field 3, Field 4...)
                    let mut compatible_id: Vec<String> = Vec::new();
                    let field_count = SetupAPI::get_field_count(&model_context);
                    if field_count > 2 {
                        for i in 3..=field_count {
                            if let Ok(compat_id) = SetupAPI::get_string_field(&model_context, i) {
                                compatible_id.push(compat_id.to_uppercase());
                            }
                        }
                    }

                    // 构建 Entry
                    hardware_entries.push(HardwareEntry {
                        desc: name.clone(),
                        arch: arch.clone(),
                        min_os_version: os_version.clone(),
                        hardware_id: hardware_id.clone(),
                        compatible_ids: compatible_id.clone(),
                    });
                }

                // 移动到下一行
                match SetupAPI::find_next_line(&mut model_context) {
                    // 更新 context
                    Ok(ctx) => model_context = ctx,
                    // 该节遍历结束，跳出 inner loop，处理下一个 section
                    Err(_) => break,
                };
            }
        }

        SetupAPI::close_inf_file(handle_inf);

        // 转换为相对路径
        let inf_path = inf_file
            .strip_prefix(base_path)
            .with_context(|| "Strip inf path prefix failed")?;

        Ok(InfInfo {
            path: inf_path.to_string_lossy().to_string(),
            class,
            date,
            version,
            signature,
            hardware: hardware_entries,
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
                    let ord = compare_version(&a.version, &b.version);
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
                ord => ord, // class 升序
            }
        });
    }
}

/// 从 Section 名称中解析架构和 OS 版本
/// 例如输入: "Realtek.NTamd64.10.0"
/// 输出: (DriverArch::NTamd64, "10.0")
/// 如果输入: "Realtek.NTx86" (无版本号)
/// 输出: (DriverArch::NTx86, "")
fn parse_section_metadata(section_name: &str) -> (DriverArch, String) {
    let parts: Vec<&str> = section_name.split('.').collect();

    let mut arch = DriverArch::Nt;
    let mut version_parts = Vec::new(); // 临时存 ["10", "0"]
    let mut found_arch = false;

    for part in parts {
        // 如果已经找到架构了，剩下的全是版本号部分
        if found_arch {
            version_parts.push(part);
            continue;
        }

        // 尝试匹配架构
        match part.to_lowercase().as_str() {
            "ntx86" => {
                arch = DriverArch::NTx86;
                found_arch = true;
            }
            "ntamd64" => {
                arch = DriverArch::NTamd64;
                found_arch = true;
            }
            "ntia64" => {
                arch = DriverArch::NTia64;
                found_arch = true;
            }
            "ntarm" => {
                arch = DriverArch::NTarm;
                found_arch = true;
            }
            "ntarm64" => {
                arch = DriverArch::NTarm64;
                found_arch = true;
            }
            _ => {
                // 既不是架构也不是版本，可能是厂商名，跳过
            }
        }
    }

    // 将版本部分拼接回字符串 "Major.Minor"
    let os_version = if version_parts.is_empty() {
        String::new() // 没有版本限制
    } else {
        version_parts.join(".") // ["10", "0"] -> "10.0"
    };

    (arch, os_version)
}

// 解析INF文件（通过按行读取）
//
// # 参数
// - `base_path`: inf 基本路径（父路径）
// - `inf_file`: inf 文件路径
//
// # 返回值
// - `Ok(InfInfo)`: 解析后的INF驱动信息
// pub fn parse_from_inf_by_line(base_path: &Path, inf_file: &Path) -> Result<InfInfo> {
//     // 打开INF文件
//     let mut file = File::open(inf_file)
//         .with_context(|| format!("Open inf file Failed: {:?}", inf_file))?;
//
//     // 读取INF文件
//     let mut buffer: Vec<u8> = Vec::new();
//     file.read_to_end(&mut buffer)
//         .with_context(|| format!("Read inf file Failed: {:?}", inf_file))?;
//
//     // 自动识别编码并以UTF-8读取
//     let result = chardet::detect(&buffer);
//     let coder = label::encoding_from_whatwg_label(chardet::charset2encoding(&result.0))
//         .with_context(|| "Detect INF file encoding failed".to_string())?;
//     let inf_content = match coder.decode(&buffer, DecoderTrap::Ignore) {
//         Ok(content) => content,
//         Err(e) => {
//             return Err(anyhow!("Decode inf file failed: {}", e));
//         }
//     };
//
//     // 去除INF内的所有 空格 与 tab符
//     let inf_content = inf_content.replace(" ", "").replace("	", "");
//
//     let mut class = String::new();
//     let mut date = String::new();
//     let mut version = String::new();
//     let mut arch: Vec<DriverArch> = Vec::new();
//     let mut hwid: Vec<String> = Vec::new();
//     let mut cid: Vec<String> = Vec::new();
//
//     // 按行读取
//     for line in inf_content.lines() {
//         // 跳过空行、注释行
//         if line.is_empty() || line.starts_with(";") {
//             continue;
//         }
//
//         // 去除行尾注释
//         let line = line.split(';').next().unwrap_or(line).trim();
//
//         // 变量替换处理
//         let line = extract_vars(line)
//             .iter()
//             .fold(line.to_string(), |acc, ver| {
//                 inf_content
//                     .get_string_center(&format!("{ver}="), "\r\n")
//                     .map(|v| acc.replace(&format!("%{ver}%"), v.trim_matches('"')))
//                     .unwrap_or(acc)
//             });
//
//         // 转换为小写
//         let lower_line = line.to_lowercase();
//
//         // 驱动类别
//         if let Some(c) = lower_line.strip_prefix("class=") {
//             // 首字母大写
//             class = c[0..1].to_uppercase() + &c[1..];
//         }
//
//         // 驱动版本、日期
//         if let Some(date_and_version) = lower_line.strip_prefix("driverver=") {
//             let (mut d, v) = date_and_version
//                 .split_once(',')
//                 .map(|(d, v)| (d.trim(), v.trim()))
//                 .unwrap_or((date_and_version, ""));
//
//             // 去掉前导非数字（例如 "Thu03/14/2002"、"Thu 03/14/2002"）
//             if let Some(pos) = d.find(|c: char| c.is_ascii_digit()) {
//                 d = &d[pos..];
//             }
//             date = match NaiveDate::parse_from_str(d, "%m/%d/%Y") {
//                 Ok(dt) => dt,
//                 Err(_) => NaiveDate::parse_from_str(d, "%Y/%m/%d")
//                     .with_context(|| format!("parse date failed: {}", d))?,
//             }
//             .format("%Y-%m-%d")
//             .to_string();
//
//             version = v.to_string();
//         }
//
//         // 驱动平台
//         if lower_line.contains(".ntx86") && !arch.contains(&DriverArch::NTx86) {
//             arch.push(DriverArch::NTx86);
//         }
//         if lower_line.contains(".ntamd64") && !arch.contains(&DriverArch::NTamd64) {
//             arch.push(DriverArch::NTamd64);
//         }
//         if lower_line.contains(".ntia64") && !arch.contains(&DriverArch::NTia64) {
//             arch.push(DriverArch::NTia64);
//         }
//         if lower_line.contains(".ntarm") && !arch.contains(&DriverArch::NTarm) {
//             arch.push(DriverArch::NTarm);
//         }
//         if lower_line.contains(".ntarm64") && !arch.contains(&DriverArch::NTarm64) {
//             arch.push(DriverArch::NTarm64);
//         }
//         if (lower_line.contains(".nt")
//             && !lower_line.contains(".ntx86")
//             && !lower_line.contains(".ntamd64")
//             && !lower_line.contains(".ntia64")
//             && !lower_line.contains(".ntarm")
//             && !lower_line.contains(".ntarm64"))
//             && !arch.contains(&DriverArch::Nt)
//         {
//             arch.push(DriverArch::Nt);
//         }
//
//         // 获取硬件ID（如果存在等于号并且逗号分隔则获取逗号之后的部分）
//         if let Some(equal_pos) = line.find('=')
//             && let Some(comma_pos) = line[equal_pos..].find(',')
//         {
//             // 获取逗号之后的部分
//             let potential_id = &line[(equal_pos + comma_pos + 1)..].trim();
//
//             // 排除关键字
//             let exclude_keywords = [
//                 "SYSWOW32",
//                 "SYSWOW64",
//                 "PROGRAMDATA",
//                 "\\X86",
//                 "\\X64",
//                 "\\AMD64",
//                 "\\I386",
//             ];
//             if exclude_keywords
//                 .iter()
//                 .any(|k| potential_id.to_uppercase().contains(k))
//             {
//                 continue;
//             }
//
//             // 逗号分隔硬件ID、兼容ID
//             let mut first_id = true;
//             for id in potential_id.split(",") {
//                 // 检查硬件ID特征（必须包含反斜杠 或 开头为星号）
//                 if id == "\\" || (!id.contains('\\') && !id.starts_with('*')) {
//                     continue;
//                 }
//
//                 // 检查是否符合硬件ID格式
//                 if !id.chars().all(|c| {
//                     c.is_ascii_alphanumeric()
//                         || c == '\\'
//                         || c == '&'
//                         || c == '_'
//                         || c == '.'
//                         || c == '-'
//                         || c == '*'
//                         || c == ':'
//                         || c == '{'
//                         || c == '}'
//                 }) {
//                     continue;
//                 }
//
//                 if first_id {
//                     if !hwid.contains(&id.to_uppercase()) {
//                         hwid.push(id.to_uppercase());
//                     }
//                     first_id = false;
//                 } else {
//                     if !cid.contains(&id.to_uppercase()) {
//                         cid.push(id.to_uppercase());
//                     }
//                 }
//             }
//         }
//     }
//
//     // 转换为相对路径
//     let inf_path = inf_file
//         .strip_prefix(base_path)
//         .with_context(|| "Strip inf path prefix failed")?;
//
//     Ok(InfInfo {
//         path: inf_path.to_string_lossy().to_string(),
//         class,
//         arch,
//         date,
//         version,
//         hwid,
//         cid,
//     })
// }
