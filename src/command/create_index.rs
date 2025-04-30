use std::collections::HashMap;
use std::error::Error;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::{fs, thread};
use crate::i18n::getLocaleText;
use crate::utils::console::{writeConsole, ConsoleType};
use crate::utils::sevenZIP::sevenZip;
use crate::utils::util::{extract_vars, getFileList, String_utils};
use crate::TEMP_PATH;
use chardet::{charset2encoding, detect};
use encoding::label::encoding_from_whatwg_label;
use encoding::DecoderTrap;
use fluent_templates::fluent_bundle::FluentValue;
use serde::{Deserialize, Serialize};

/// INF驱动信息
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct InfInfo {
    /// 驱动路径
    pub(crate) Path: String,
    /// 驱动INF文件名
    pub(crate) Inf: String,
    /// 驱动类别
    pub(crate) Class: String,
    /// 驱动位宽
    pub(crate) Arch: Vec<String>,
    /// 驱动日期
    pub(crate) Date: String,
    /// 驱动版本
    pub(crate) Version: String,
    /// 驱动硬件id列表
    pub(crate) DriverList: Vec<String>,
}

impl InfInfo {
    /// 解析INF文件
    /// # 参数
    /// 1. inf 基本路径（父路径）
    /// 2. inf 文件路径
    pub fn parsingInfFile(basePath: &Path, infFile: &Path) -> Result<InfInfo, Box<dyn Error>> {
        lazy_static! {
            // 系统架构
            static ref SYSTEMARCH: [&'static str; 5] = ["NTx86", "NTia64","NTamd64", "NTarm", "NTarm64"];
        }

        // 读取INF文件
        let mut file = File::open(infFile)?;
        let mut fileBuf: Vec<u8> = Vec::new();
        file.read_to_end(&mut fileBuf)?;

        // 自动识别编码并以UTF-8读取
        let result = detect(&fileBuf);
        let coder = encoding_from_whatwg_label(charset2encoding(&result.0)).ok_or("Failed encoding")?;
        let infContent = coder.decode(&fileBuf, DecoderTrap::Ignore)?;

        // 去除INF内的所有 空格 与 tab符
        let infContent = infContent.replace(" ", "").replace("	", "");

        let mut idList: Vec<String> = Vec::new();

        let mut Class = String::new();
        let mut Date = String::new();
        let mut Version = String::new();
        let mut Arch: Vec<String> = Vec::new();

        // 按行读取
        for line in infContent.lines() {
            // 空行、行首注释
            if line.is_empty() || line.starts_with(";") { continue; }
            // 行尾注释
            let line = line.split(';').next().unwrap_or(line).trim();

            // 驱动类别
            if let Some(class) = line.strip_prefix("Class=") {
                Class = String::from(class);
            }
            // 驱动版本、日期
            if let Some(dateAndVersion) = line.strip_prefix("DriverVer=") {
                // 变量替换处理
                let dateAndVersion = extract_vars(&*dateAndVersion).iter().fold(
                    dateAndVersion.to_string(),
                    |acc, ver| {
                        infContent.get_string_center(&format!("{ver}="), "\r\n")
                            .map(|v| acc.replace(&format!("%{ver}%"), v.trim_matches('"')))
                            .unwrap_or(acc)
                    }
                );

                let (date, version) = dateAndVersion.split_once(',')
                    .map(|(d, v)| (d.trim(), v.trim()))
                    .unwrap_or((&*dateAndVersion, ""));
                Date = date.to_string();
                Version = version.to_string();
            }
            // 驱动平台
            for item in SYSTEMARCH.iter() {
                if line.to_uppercase().contains(&*format!(".{}", item).to_uppercase()) && !Arch.contains(&item.to_string()) {
                    Arch.push(item.to_string());
                }
            }
            // 获取硬件ID
            if let Some(equal_pos) = line.find('=') {
                if let Some(comma_pos) = line[equal_pos..].find(',') {
                    // 获取逗号之后的部分
                    let potential_id = &line[(equal_pos + comma_pos + 1)..].trim();
                    // 逗号分隔多个硬件ID
                    for id in potential_id.split(",") {
                        // 检查是否包含反斜杠或开头是否为星号
                        if id == "\\" || (!id.contains('\\') && !id.starts_with('*')) {
                            continue;
                        }
                        // 检查是否符合硬件ID格式
                        if !id.chars().all(|c| c.is_ascii_alphanumeric() || c == '\\' || c == '&' || c == '_' || c == '.' || c == '-' || c == '*') {
                            continue;
                        }
                        // 转为大写
                        let id = id.to_uppercase();
                        if !idList.contains(&id) {
                            idList.push(id);
                        }
                    }
                }
            }
        }

        // 获取驱动文件相对路径
        let parentPath = infFile.parent().unwrap().strip_prefix(basePath)?;

        Ok(InfInfo {
            Path: parentPath.to_str().unwrap().parse().unwrap(),
            Inf: infFile
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .parse()
                .unwrap(),
            Class,
            Arch,
            Date,
            Version,
            DriverList: idList,
        })
    }

    /// 解析INF文件列表（多线程）
    /// 解析失败的INF将自动跳过
    /// # 参数
    /// 1. inf 基本路径（父路径）
    /// 2. inf 文件路径列表
    pub fn parsingInfFileList(basePath: &Path, infFileList: &[PathBuf]) -> Vec<InfInfo> {
        let (tx, rx) = mpsc::channel();
        for item in infFileList.iter() {
            let basePath = basePath.to_path_buf();
            let infFile = item.clone();
            let tx1 = mpsc::Sender::clone(&tx);
            thread::spawn(move || {
                if let Ok(infInfo) = InfInfo::parsingInfFile(&basePath, &infFile) {
                    tx1.send(infInfo).unwrap();
                }
            });
        }
        // 释放接受者，否则会卡住造成死锁
        drop(tx);
        // 接收数据并转为INF信息数组
        let infInfoList = rx.iter().collect::<Vec<InfInfo>>();
        infInfoList
    }

    /// 保存INF数据（通过JSON）
    /// #参数
    /// 1. INF列表
    /// 2. 索引文件保存路径
    pub fn saveIndexFromJson(Data: &[InfInfo], savaPath: &Path) -> Result<(), Box<dyn Error>> {
        let json = serde_json::to_string(&Data)?;
        fs::write(savaPath, json)?;
        Ok(())
    }

    /// 解析索引数据
    /// # 参数
    /// 1. 索引文件路径
    pub fn parsingIndex(indexPath: &Path) -> Result<Vec<InfInfo>, Box<dyn Error>> {
        let mut indexFile = File::open(indexPath)?;
        let mut indexContent = String::new();
        indexFile.read_to_string(&mut indexContent)?;
        let json: Vec<InfInfo> = serde_json::from_str(&indexContent)?;
        Ok(json)
    }
}

pub fn createIndex(drivePath: &Path, password: Option<&str>, saveIndexPath: &Path) {
    writeConsole(ConsoleType::Info, &getLocaleText("processing", None));

    let zip = sevenZip::new().unwrap();

    // INF文件父路径
    let infParentPath;
    // INF文件列表
    let infList;
    // 保存索引路径
    let indexPath;

    if drivePath.is_dir() {
        // 从驱动目录中创建索引文件
        infList = getFileList(drivePath, "*.inf").unwrap();
        infParentPath = drivePath.to_path_buf();
        // 如果输入的索引路径是相对路径，则令实际路径为驱动目录所在路径
        indexPath = if saveIndexPath.is_relative() {
            drivePath.join(saveIndexPath)
        } else {
            saveIndexPath.to_path_buf()
        };
    } else {
        // 从文件中创建索引文件
        infParentPath = TEMP_PATH.join(drivePath.file_stem().unwrap());
        // 解压INF文件
        if !zip.extractFilesFromPathRecurseSubdirectories(drivePath, password, "*.inf", &infParentPath).unwrap() {
            writeConsole(
                ConsoleType::Err,
                &getLocaleText("driver-unzip-failed", None),
            );
            return;
        }

        infList = getFileList(&infParentPath, "*.inf").unwrap();
        // 如果输入的索引路径是相对路径，则令实际实际为驱动包所在路径
        indexPath = if saveIndexPath.is_relative() {
            drivePath.parent().unwrap().join(saveIndexPath)
        } else {
            saveIndexPath.to_path_buf()
        };
    }

    if infList.is_empty() {
        writeConsole(ConsoleType::Err, &getLocaleText("no-inf-find", None));
        return;
    }

    let mut infInfoList: Vec<InfInfo> = Vec::new();

    let (mut successCount, mut ErrorCount, mut blankCount) = (0, 0, 0);

    // 遍历INF文件
    for item in infList.iter() {
        let arg = hash_map!("path".to_string() => item.to_str().unwrap().into());
        if let Ok(currentInfo) = InfInfo::parsingInfFile(&infParentPath, item) {
            if currentInfo.DriverList.is_empty() {
                blankCount += 1;
                writeConsole(
                    ConsoleType::Warning,
                    &getLocaleText("no-hardware", Some(&arg)),
                );
                continue;
            }
            successCount += 1;
            infInfoList.push(currentInfo);
        } else {
            ErrorCount += 1;
            writeConsole(
                ConsoleType::Err,
                &getLocaleText("inf-parsing-err", Some(&arg)),
            );
        }
    }

    if let Err(_e) = InfInfo::saveIndexFromJson(&infInfoList, &indexPath) {
        writeConsole(ConsoleType::Err, &getLocaleText("index-save-failed", None));
        return;
    }
    let arg: HashMap<String, FluentValue> = hash_map!(
        "total".to_string() => infList.len().into(),
        "success".to_string() => successCount.to_string().into(),
        "error".to_string() => ErrorCount.to_string().into(),
        "blankCount".to_string() => blankCount.to_string().into(),
    );
    writeConsole(ConsoleType::Info, &getLocaleText("total-info", Some(&arg)));
    let arg: HashMap<String, FluentValue> = hash_map!("path".to_string() => indexPath.to_str().unwrap().into());
    writeConsole(ConsoleType::Success, &getLocaleText("saveInfo", Some(&arg)));
}
