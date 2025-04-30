use std::error::Error;
use std::path::{Path, PathBuf};
use std::process::Command;
use crate::utils::util::{writeEmbedFile, String_utils};
use crate::TEMP_PATH;
use windows_version::OsVersion;

/// 硬件信息
#[derive(Debug, Clone, Eq)]
pub struct HwID {
    /// 设备实例路径
    pub(crate) DeviceInstancePath: String,
    /// 显示名称
    pub(crate) Name: String,
    /// 硬件id
    pub(crate) HardwareIDs: Vec<String>,
    /// 兼容id
    pub(crate) CompatibleIDs: Vec<String>,
}

impl PartialEq for HwID {
    fn eq(&self, other: &Self) -> bool {
        self.DeviceInstancePath == other.DeviceInstancePath
    }
}

/// Devcon操作类
/// # 如何获取Devcon？
/// [WDK 下载](https://docs.microsoft.com/zh-cn/windows-hardware/drivers/download-the-wdk)
pub struct Devcon {
    devconPath: PathBuf,
}

impl Devcon {
    /// 初始化
    pub fn new() -> Result<Devcon, Box<dyn Error>> {
        let devconPath = TEMP_PATH.join("devcon.exe");
        writeEmbedFile(if OsVersion::current().major == 5 { "devcon-nt5.exe" } else { "devcon.exe" }, &devconPath)?;
        Ok(Devcon { devconPath })
    }

    /// 获取真实硬件id信息
    /// #参数
    /// 1. 驱动类别（注意：只能获取已安装驱动的设备）
    pub fn getRealIdInfo<T1>(&self, driveClass: T1) -> Result<Vec<HwID>, Box<dyn Error>>
    where
        T1: Into<Option<String>>,
    {
        let driveClass = driveClass.into();
        let hwidType = if driveClass.is_some() {
            format!("={}", &driveClass.unwrap())
        } else {
            "*".to_string()
        };
        let output = Command::new(&self.devconPath)
            .arg("hwids")
            .arg(hwidType)
            .output()?;

        let content = String::from_utf8_lossy(&output.stdout);

        // 将 Name 与 Hardware IDs 分离
        let content = content.replace("     Hardware IDs:", "\r\n    Hardware IDs:");
        // 将 Name 与 Compatible IDs 分离，并加上空的Hardware IDs
        let content = content.replace(
            "     Compatible IDs:",
            "\r\n    Hardware IDs:\r\n    Compatible IDs:",
        );

        const DELIMITER: &str = "|";
        const SUBDELIMITER: &str = ",";

        // 将输出的运行状态转换为每个项目一行以便读取
        let contentLine = content.replace("\r\n        ", SUBDELIMITER);
        let contentLine = contentLine.replace("\r\n    ", DELIMITER);
        let contentLine = contentLine.replace("  ", "");
        let contentLine = contentLine.replace("\r\n", &format!("{}\r\n", DELIMITER));

        let mut HwIDList: Vec<HwID> = Vec::new();
        // 通过换行符分割遍历
        for item in contentLine.lines() {
            // 获取设备实例路径
            let DeviceInstancePath = item
                .to_string()
                .get_string_left(DELIMITER)
                .unwrap_or_else(|_| "".to_string());

            // 获取显示名称
            let name = item
                .to_string()
                .get_string_center("Name:", DELIMITER)
                .unwrap_or_else(|_| "".to_string());

            // 获取硬件id
            let hardwareIDs = item
                .to_string()
                .get_string_center("Hardware IDs:", DELIMITER)
                .unwrap_or_else(|_| "".to_string())
                .replace(DELIMITER, "");
            let hardwareIDList: Vec<String> = hardwareIDs
                .split(SUBDELIMITER)
                .filter(|&hardwareID| !hardwareID.is_empty())
                .map(|hardwareID| hardwareID.to_string())
                .collect();

            // 获取兼容id
            let CompatibleIDs = item
                .to_string()
                .get_string_right("Compatible IDs:")
                .unwrap_or_else(|_| "".to_string())
                .replace(DELIMITER, "");
            let CompatibleIDList: Vec<String> = CompatibleIDs
                .split(SUBDELIMITER)
                .filter(|&CompatibleID| !CompatibleID.is_empty())
                .map(|CompatibleID| CompatibleID.to_string())
                .collect();

            HwIDList.push(HwID {
                DeviceInstancePath,
                Name: name,
                HardwareIDs: hardwareIDList,
                CompatibleIDs: CompatibleIDList,
            });
        }
        Ok(HwIDList)
    }

    /// 获取有问题的硬件设备实例路径
    pub fn getProblemDeviceInstancePath(&self) -> Result<Vec<String>, Box<dyn Error>> {
        // pnputil /enum-devices /problem /ids
        // 列出设备的运行状态
        let output = Command::new(&self.devconPath)
            .arg("status")
            .arg("*")
            .output()?;
        let content = String::from_utf8_lossy(&output.stdout);

        const DELIMITER: &str = "|";

        // 将输出的运行状态转换为一行以便读取
        let contentLine = content.replace("\r\n    ", DELIMITER);

        let mut problemIdList: Vec<String> = Vec::new();
        // 通过换行符分割遍历
        for item in contentLine.lines() {
            if item.contains("problem") {
                let id = item.split(DELIMITER).next().unwrap_or("");
                if !id.is_empty() {
                    problemIdList.push(String::from(id));
                }
            }
        }
        Ok(problemIdList)
    }

    /// 获取有问题的硬件id信息
    /// # 参数
    /// 1. 真实硬件id信息
    pub fn getProblemIdInfo(&self, realIdInfo: Vec<HwID>) -> Result<Vec<HwID>, Box<dyn Error>> {
        // 获取有问题的硬件设备实例路径
        let problemIdList = &self.getProblemDeviceInstancePath()?;

        let mut problemIdInfoList: Vec<HwID> = Vec::new();

        // 遍历有问题的硬件设备实例路径
        for problemId in problemIdList {
            // 遍历获取真实硬件id信息
            // let problemIdInfoList: Vec<HwID> = realIdInfo.into_iter().filter(|idInfo| problemId.to_lowercase() == idInfo.DeviceInstancePath.to_lowercase()).collect();
            for realIdItem in realIdInfo.iter() {
                if problemId.to_lowercase() == realIdItem.DeviceInstancePath.to_lowercase() {
                    problemIdInfoList.push(realIdItem.clone());
                    break;
                }
            }
        }
        Ok(problemIdInfoList)
    }

    /// 加载驱动
    /// # 参数
    /// 1. INF文件路径
    /// 2. 硬件ID（不是设备实例路径）
    pub fn loadDriver(&self, infPath: &Path, hwid: &str) -> Result<bool, Box<dyn Error>> {
        // 不要用 install 命令
        let output = Command::new(&self.devconPath)
            .arg("update")
            .arg(infPath)
            .arg(hwid)
            .output()?;
        let content = String::from_utf8_lossy(&output.stdout);
        Ok(content.contains("successfully"))
    }

    /// 扫描以发现新的硬件
    pub fn rescan(&self) -> Result<bool, Box<dyn Error>> {
        let output = Command::new(&self.devconPath).arg("rescan").output()?;
        let content = String::from_utf8_lossy(&output.stdout);
        Ok(content.contains("completed"))
    }

    /// 卸载设备
    /// # 参数
    /// 1. 硬件ID
    pub fn removeDevice(&self, id: &str) -> Result<bool, Box<dyn Error>> {
        let output = Command::new(&self.devconPath)
            .arg("remove")
            .arg(id)
            .output()?;
        let content = String::from_utf8_lossy(&output.stdout);
        Ok(content.contains("were removed"))
    }
}
