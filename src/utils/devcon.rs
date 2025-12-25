pub(crate) use crate::hardware::HardwareInfo;
use crate::utils::utils::{write_embed_file, String_utils};
use crate::TEMP_PATH;
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::process::Command;
use windows_version::OsVersion;

/// Devcon操作类
/// 如何获取Devcon？
/// [WDK 下载](https://docs.microsoft.com/zh-cn/windows-hardware/drivers/download-the-wdk)
///
/// # 注意
/// NT5 系统需要使用 devcon-v5.2.3790 版本
pub struct Devcon {
    devcon_program: PathBuf,
}

impl Devcon {
    /// 初始化
    pub fn new() -> Result<Devcon> {
        let devcon_path = TEMP_PATH.join("devcon.exe");
        if !devcon_path.exists() {
            write_embed_file(
                if OsVersion::current().major == 5 {
                    "devcon-nt5.exe"
                } else {
                    "devcon.exe"
                },
                &devcon_path,
            )
            .with_context(|| "Write devcon.exe to temp path failed")?;
        }

        Ok(Devcon {
            devcon_program: devcon_path,
        })
    }

    /// 获取真实硬件设备信息
    ///
    /// # 参数
    /// - `drive_class` 驱动类别（注意：只能获取已安装驱动的设备）
    ///
    /// # 返回值
    /// - `Ok(Vec<HwID>)`: 真实硬件id信息列表
    pub fn get_hardware_device_info(&self, drive_class: Option<&str>) -> Result<Vec<HardwareInfo>> {
        let output = Command::new(&self.devcon_program)
            .arg("hwids")
            .arg(if let Some(drive_class) = drive_class {
                format!("={}", &drive_class)
            } else {
                "*".to_string()
            })
            .output()
            .with_context(|| "get hardware device info failed")?;

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
        let content_line = content.replace("\r\n        ", SUBDELIMITER);
        let content_line = content_line.replace("\r\n    ", DELIMITER);
        let content_line = content_line.replace("  ", "");
        let content_line = content_line.replace("\r\n", &format!("{}\r\n", DELIMITER));

        let mut hwid_list: Vec<HardwareInfo> = Vec::new();

        // 通过换行符分割遍历
        for item in content_line.lines() {
            // 获取设备实例路径
            let device_instance_path = item
                .to_string()
                .get_string_left(DELIMITER)
                .unwrap_or_else(|_| "".to_string());

            // 获取显示名称
            let name = item
                .to_string()
                .get_string_center("Name: ", DELIMITER)
                .unwrap_or_else(|_| "".to_string());

            // 获取硬件id
            let hardware_ids = item
                .to_string()
                .get_string_center("Hardware IDs:", DELIMITER)
                .unwrap_or_else(|_| "".to_string())
                .replace(DELIMITER, "");
            let hardware_id_List: Vec<String> = hardware_ids
                .split(SUBDELIMITER)
                .filter(|&hardwareID| !hardwareID.is_empty())
                .map(|hardwareID| hardwareID.to_string())
                .collect();

            // 获取兼容id
            let compatible_ids = item
                .to_string()
                .get_string_right("Compatible IDs:")
                .unwrap_or_else(|_| "".to_string())
                .replace(DELIMITER, "");
            let compatible_id_list: Vec<String> = compatible_ids
                .split(SUBDELIMITER)
                .filter(|&CompatibleID| !CompatibleID.is_empty())
                .map(|CompatibleID| CompatibleID.to_string())
                .collect();

            hwid_list.push(HardwareInfo {
                device_instance_path,
                name,
                hardware_id: hardware_id_List,
                compatible_id: compatible_id_list,
            });
        }
        Ok(hwid_list)
    }

    /// 获取有问题的硬件设备信息
    ///
    /// # 参数
    /// - `hardware_device_info` - 真实硬件id信息列表
    /// - `drive_class` 驱动类别（注意：只能获取已安装驱动的设备）
    ///
    /// # 返回值
    /// - `Ok(Vec<HwID>)`: 有问题的硬件id信息列表
    /// - `Err(e)`: 失败则返回错误信息
    pub fn get_problem_device_info(&self, drive_class: Option<&str>) -> Result<Vec<HardwareInfo>> {
        // 获取真实硬件设备信息
        let hardware_device_info = self.get_hardware_device_info(drive_class)?;

        // 获取有问题的硬件设备实例路径
        let problem_device_list = &self
            .get_problem_device_instance_path()
            .with_context(|| "get problem device instance path failed")?;

        let mut problem_device_info_list = Vec::new();

        // 遍历有问题的硬件设备实例路径
        for problem_device_info in problem_device_list {
            // 遍历获取真实硬件id信息
            for real_device_info in hardware_device_info.iter() {
                if problem_device_info.eq_ignore_ascii_case(&real_device_info.device_instance_path)
                {
                    problem_device_info_list.push(real_device_info.clone());
                    break;
                }
            }
        }

        Ok(problem_device_info_list)
    }

    /// 获取有问题的硬件设备实例路径
    ///
    /// # 返回值
    /// - `Ok(Vec<String>)`: 有问题的硬件设备实例路径列表
    /// - `Err(e)`: 失败则返回错误信息
    fn get_problem_device_instance_path(&self) -> Result<Vec<String>> {
        // pnputil /enum-devices /problem /ids
        // 列出设备的运行状态
        let output = Command::new(&self.devcon_program)
            .arg("status")
            .arg("*")
            .output()
            .with_context(|| "get problem device instance path failed")?;
        let content = String::from_utf8_lossy(&output.stdout);

        const DELIMITER: &str = "|";

        // 将输出的运行状态转换为一行以便读取
        let content_line = content.replace("\r\n    ", DELIMITER);

        let mut problem_device_list = Vec::new();

        // 通过换行符分割遍历
        for item in content_line.lines() {
            if item.contains("problem") {
                let id = item.split(DELIMITER).next().unwrap_or("");
                if !id.is_empty() {
                    problem_device_list.push(String::from(id));
                }
            }
        }
        Ok(problem_device_list)
    }

    /// 加载指定INF文件的驱动程序到指定的硬件ID。
    ///
    /// # 参数
    /// - `hwid` - 硬件ID（不是设备实例路径）
    /// - `infPath` - INF文件路径
    ///
    /// # 返回值
    /// - `Ok(bool)`: 如果驱动加载成功，则返回 `true`；否则返回 `false`。
    pub fn install_driver(&self, hardware_id: &str, inf_path: &Path) -> Result<bool> {
        // 不要用 install 命令
        let output = Command::new(&self.devcon_program)
            .arg("update")
            .arg(inf_path)
            .arg(hardware_id)
            .output()
            .with_context(|| format!("install driver {} failed", hardware_id))?;
        Ok(String::from_utf8_lossy(&output.stdout).contains("successfully"))
    }

    /// 扫描以发现新的硬件
    ///
    /// # 返回值
    /// - `Ok(bool)`: 如果扫描成功，则返回 `true`；否则返回 `false`。
    pub fn rescan(&self) -> Result<bool> {
        let output = Command::new(&self.devcon_program)
            .arg("rescan")
            .output()
            .with_context(|| "rescan failed")?;
        Ok(String::from_utf8_lossy(&output.stdout).contains("completed"))
    }

    /// 卸载设备
    ///
    /// # 参数
    /// - `id`: 硬件ID（不是设备实例路径）
    ///
    /// # 返回值
    /// - `Ok(bool)`: 如果设备卸载成功，则返回 `true`；否则返回 `false`。
    pub fn remove_device(&self, id: &str) -> Result<bool> {
        let output = Command::new(&self.devcon_program)
            .arg("remove")
            .arg(id)
            .output()
            .with_context(|| format!("remove device {} failed", id))?;
        Ok(String::from_utf8_lossy(&output.stdout).contains("were removed"))
    }
}
