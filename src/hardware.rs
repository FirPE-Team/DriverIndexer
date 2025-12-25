use crate::utils::setupapi::SetupAPI;
use anyhow::{Context, Result};
use std::path::Path;
use windows::core::{BOOL, GUID, HSTRING};
use windows::Win32::Devices::DeviceAndDriverInstallation::{
    CM_Get_DevNode_Status, UpdateDriverForPlugAndPlayDevicesW, CM_DEVNODE_STATUS_FLAGS, CM_PROB,
    CM_PROB_DISABLED, CM_PROB_FAILED_INSTALL, CM_PROB_NOT_CONFIGURED, CM_PROB_REINSTALL, CR_SUCCESS,
    DN_HAS_PROBLEM, INSTALLFLAG_FORCE, UPDATEDRIVERFORPLUGANDPLAYDEVICES_FLAGS,
};
use windows::{
    core::PCWSTR,
    Win32::Devices::DeviceAndDriverInstallation::{
        SetupDiDestroyDeviceInfoList, SetupDiEnumDeviceInfo, SetupDiGetClassDevsW, SetupDiGetDeviceInstanceIdW, SetupDiGetDeviceRegistryPropertyW,
        DIGCF_ALLCLASSES, DIGCF_PRESENT, HDEVINFO, SETUP_DI_REGISTRY_PROPERTY,
        SPDRP_COMPATIBLEIDS, SPDRP_DEVICEDESC, SPDRP_FRIENDLYNAME,
        SPDRP_HARDWAREID, SP_DEVINFO_DATA,
    },
};

/// 硬件信息
#[derive(Debug, Clone, Eq)]
pub struct HardwareInfo {
    /// 设备实例路径
    pub(crate) device_instance_path: String,
    /// 显示名称
    pub(crate) name: String,
    /// 硬件ID
    pub(crate) hardware_id: Vec<String>,
    /// 兼容ID
    pub(crate) compatible_id: Vec<String>,
}

/// 实现 PartialEq  trait，根据设备实例路径比较硬件信息是否相等
impl PartialEq for HardwareInfo {
    fn eq(&self, other: &Self) -> bool {
        self.device_instance_path == other.device_instance_path
    }
}

/// 枚举所有硬件设备
/// [参考资料](https://docs.microsoft.com/zh-cn/windows/win32/api/setupapi/nf-setupapi-setupdigetclassdevsexa)
///
/// # 参数
/// - `class_name`: 可选的设备类名 (例如 "USB", "SCSI")。
/// - `only_missing`: 是否只返回缺失/未配置的设备
///
/// # 注意
/// - `class_name`参数仅能返回已安装驱动设备，在驱动安装之前无法确定硬件类别
///
/// # 返回值
/// - `Ok(Vec<HardwareInfo>)`: 成功，返回硬件信息列表
/// - `Err(anyhow::Error)`: 失败，包含错误信息
pub fn enumerate_hardware(
    class_name: Option<&str>,
    only_missing: bool,
) -> Result<Vec<HardwareInfo>> {
    let mut result_list = Vec::new();

    // 获取设备集合句柄 (所有已存在的设备)
    let (guids_to_scan, base_flags) = if let Some(name) = class_name {
        // 指定了类名
        let guids = SetupAPI::get_class_guids_from_name(name)
            .with_context(|| format!("Failed to get class GUIDs for class name: {:?}", name))?;

        if guids.is_empty() {
            // 如果名字查不到任何 GUID，直接返回空列表
            return Ok(Vec::new());
        }

        // DIGCF_PRESENT: 只返回当前已连接/存在的设备 (不包含未连接的历史设备)
        (
            guids.into_iter().map(Some).collect::<Vec<_>>(),
            DIGCF_PRESENT,
        )
    } else {
        // 未指定类名 (None)
        // DIGCF_ALLCLASSES: 忽略 ClassGuid 参数，返回所有类别的设备
        // DIGCF_PRESENT: 只返回当前已连接/存在的设备 (不包含未连接的历史设备)
        (vec![None], DIGCF_ALLCLASSES | DIGCF_PRESENT)
    };

    // 外层循环：遍历每一个 GUID 集合
    for guid_opt in guids_to_scan {
        // 处理 GUID 指针：如果有 GUID 则取地址，否则传 null
        let guid_ptr = match guid_opt.as_ref() {
            Some(g) => g as *const GUID,
            None => std::ptr::null(),
        };

        // 获取设备集合句柄
        let device_info_set = unsafe {
            SetupDiGetClassDevsW(Some(guid_ptr), PCWSTR::null(), None, base_flags)
                .with_context(|| "Failed to get device info set")?
        };

        let mut dev_info_data = SP_DEVINFO_DATA {
            cbSize: std::mem::size_of::<SP_DEVINFO_DATA>() as u32,
            ..Default::default()
        };

        let mut index = 0;

        // 内层循环：遍历该集合下的设备
        loop {
            // 这里的 unsafe 调用如果不成功 (返回 false/Err)，说明枚举完毕
            if unsafe { SetupDiEnumDeviceInfo(device_info_set, index, &mut dev_info_data) }.is_err()
            {
                break;
            }

            // 获取设备实例路径 (Instance ID)
            let instance_path =
                get_device_instance_id(device_info_set, &mut dev_info_data).unwrap_or_default();

            // 获取名称
            let name = get_device_name(device_info_set, &mut dev_info_data);

            // 获取状态
            if only_missing && !is_driver_missing(dev_info_data.DevInst) {
                index += 1;
                continue;
            }

            // 获取硬件 ID
            let hardware_ids = get_device_property_string_list(
                device_info_set,
                &mut dev_info_data,
                SPDRP_HARDWAREID,
            );

            // 获取兼容 ID
            let compatible_ids = get_device_property_string_list(
                device_info_set,
                &mut dev_info_data,
                SPDRP_COMPATIBLEIDS,
            );

            result_list.push(HardwareInfo {
                device_instance_path: instance_path,
                name,
                hardware_id: hardware_ids,
                compatible_id: compatible_ids,
            });

            index += 1;
        }

        // 释放当前 GUID 对应的设备集句柄
        unsafe {
            let _ = SetupDiDestroyDeviceInfoList(device_info_set);
        }
    }

    // let device_info_set = unsafe {
    //     SetupDiGetClassDevsW(None, PCWSTR::null(), None, DIGCF_ALLCLASSES | DIGCF_PRESENT)
    //         .with_context(|| "Failed to get device info set")?
    // };
    //
    // let mut dev_info_data = SP_DEVINFO_DATA {
    //     cbSize: std::mem::size_of::<SP_DEVINFO_DATA>() as u32,
    //     ..Default::default()
    // };
    //
    // let mut index = 0;
    //
    // // 遍历设备
    // loop {
    //     if unsafe { SetupDiEnumDeviceInfo(device_info_set, index, &mut dev_info_data) }.is_err() {
    //         // 遍历结束 (ERROR_NO_MORE_ITEMS)
    //         break;
    //     }
    //
    //     // 获取设备实例路径 (Instance ID)
    //     let instance_path =
    //         get_device_instance_id(device_info_set, &mut dev_info_data).unwrap_or_default();
    //
    //     // 获取名称 (优先取 FriendlyName，如果没有则取 DeviceDesc)
    //     let name = get_device_name(device_info_set, &mut dev_info_data);
    //
    //     // 获取硬件 ID (REG_MULTI_SZ)
    //     let hardware_ids =
    //         get_device_property_string_list(device_info_set, &mut dev_info_data, SPDRP_HARDWAREID);
    //
    //     // 获取兼容 ID (REG_MULTI_SZ)
    //     let compatible_ids = get_device_property_string_list(
    //         device_info_set,
    //         &mut dev_info_data,
    //         SPDRP_COMPATIBLEIDS,
    //     );
    //
    //     result_list.push(HardwareInfo {
    //         device_instance_path: instance_path,
    //         name,
    //         hardware_id: hardware_ids,
    //         compatible_id: compatible_ids,
    //     });
    //
    //     index += 1;
    // }
    //
    // // 释放资源
    // unsafe {
    //     let _ = SetupDiDestroyDeviceInfoList(device_info_set);
    // }

    Ok(result_list)
}

/// 获取设备实例路径 (Instance ID)
///
/// # 参数
///
/// - `dev_info`: 设备信息集合句柄
/// - `dev_data`: 设备信息数据结构体指针
///
/// # 返回值
///
/// - `Some<String>`: 设备实例路径字符串
/// - `None`: 获取失败
fn get_device_instance_id(dev_info: HDEVINFO, dev_data: &mut SP_DEVINFO_DATA) -> Option<String> {
    let mut required_size = 0;

    // 获取长度
    unsafe {
        let _ = SetupDiGetDeviceInstanceIdW(dev_info, dev_data, None, Some(&mut required_size));
    }
    if required_size == 0 {
        return None;
    }

    let mut buffer = vec![0u16; required_size as usize];

    // 获取数据
    let success = unsafe {
        SetupDiGetDeviceInstanceIdW(
            dev_info,
            dev_data,
            Some(buffer.as_mut_slice()),
            Some(&mut required_size),
        )
    };

    if success.is_ok() {
        // 移除末尾的 null 终止符
        if let Some(&0) = buffer.last() {
            buffer.pop();
        }
        String::from_utf16(&buffer).ok()
    } else {
        None
    }
}

/// 在给定 INF 文件和 硬件 ID 的情况下，为与硬件 ID 匹配的设备安装更新的驱动程序。
/// [相关文档](https://docs.microsoft.com/zh-cn/windows/win32/api/newdev/nf-newdev-updatedriverforplugandplaydevicesa?redirectedfrom=MSDN)
///
/// # 参数
/// - `hwid`: 硬件标识符以匹配计算机上的现有设备
/// - `inf_path`: INF 文件的完整路径文件名
/// - `force`: 是否强制安装驱动程序
///
/// # 返回值
/// 如果函数成功，则返回 `true`；否则返回 `false`。
pub fn update_driver_for_plug_and_play_devices(
    hardware_id: &str,
    inf_path: &Path,
    force: bool,
) -> std::result::Result<(), windows::core::Error> {
    // 将硬件 ID 和 INF 文件路径转换为宽字符串
    let hardware_id_w = HSTRING::from(hardware_id);
    let inf_path_w = HSTRING::from(inf_path);

    // 调用 Windows API 函数 UpdateDriverForPlugAndPlayDevicesW 进行驱动程序更新
    let mut isReboot = BOOL(0);
    unsafe {
        UpdateDriverForPlugAndPlayDevicesW(
            None,
            PCWSTR(hardware_id_w.as_ptr()),
            PCWSTR(inf_path_w.as_ptr()),
            if force {
                INSTALLFLAG_FORCE
            } else {
                UPDATEDRIVERFORPLUGANDPLAYDEVICES_FLAGS(0)
            },
            Some(&mut isReboot),
        )
    }
}

/// 获取设备名称 (FriendlyName fallback to DeviceDesc)
///
/// # 参数
///
/// - `dev_info`: 设备信息集合句柄
/// - `dev_data`: 设备信息数据结构体指针
///
/// # 返回值
///
/// - `String`: 设备名称字符串
///
/// # 说明
///
/// 该函数尝试获取设备的友好名称 (FriendlyName)，如果不存在则回退到设备描述 (DeviceDesc)。
fn get_device_name(dev_info: HDEVINFO, dev_data: &mut SP_DEVINFO_DATA) -> String {
    // 尝试获取 FriendlyName (友好名称，例如 "Intel(R) Wi-Fi 6 AX200")
    if let Some(name) = get_device_property_string(dev_info, dev_data, SPDRP_FRIENDLYNAME) {
        return name;
    }

    // 如果没有 FriendlyName，获取 DeviceDesc (设备描述，例如 "Network Controller")
    get_device_property_string(dev_info, dev_data, SPDRP_DEVICEDESC)
        .unwrap_or_else(|| "Unknown Device".to_string())
}

/// 获取单个字符串类型的属性 (REG_SZ)
///
/// # 参数
///
/// - `dev_info`: 设备信息集合句柄
/// - `dev_data`: 设备信息数据结构体指针
/// - `property`: 设备属性类型 (SPDRP_*)
///
/// # 返回值
///
/// - `Some<String>`: 属性字符串
/// - `None`: 获取失败
///
/// # 说明
///
/// 该函数用于获取设备的单个字符串类型属性 (REG_SZ)，例如 FriendlyName、DeviceDesc 等。
fn get_device_property_string(
    dev_info: HDEVINFO,
    dev_data: &mut SP_DEVINFO_DATA,
    property: SETUP_DI_REGISTRY_PROPERTY, // SPDRP_*
) -> Option<String> {
    let mut required_size = 0;
    let mut prop_type = 0;

    // 获取大小
    unsafe {
        let _ = SetupDiGetDeviceRegistryPropertyW(
            dev_info,
            dev_data,
            property,
            Some(&mut prop_type),
            None,
            Some(&mut required_size),
        );
    }

    if required_size == 0 {
        return None;
    }

    let mut buffer = vec![0u8; required_size as usize];

    // 获取数据
    let success = unsafe {
        SetupDiGetDeviceRegistryPropertyW(
            dev_info,
            dev_data,
            property,
            Some(&mut prop_type),
            Some(buffer.as_mut_slice()),
            Some(&mut required_size),
        )
    };

    if success.is_ok() {
        // 将字节转换为 u16 (UTF-16)
        let wchars: Vec<u16> = buffer
            .chunks_exact(2)
            .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
            .collect();

        // 转为 String 并移除末尾 null
        let mut s = String::from_utf16_lossy(&wchars);
        if s.ends_with('\0') {
            s.truncate(s.len() - 1);
        }
        Some(s)
    } else {
        None
    }
}

/// 获取字符串列表类型的属性 (REG_MULTI_SZ)
///
/// # 参数
///
/// - `dev_info`: 设备信息集合句柄
/// - `dev_data`: 设备信息数据结构体指针
/// - `property`: 设备属性类型 (SPDRP_HARDWAREID or SPDRP_COMPATIBLEIDS)
///
/// # 返回值
///
/// - `Vec<String>`: 属性字符串列表
///
/// # 说明
///
/// 该函数用于获取设备的字符串列表类型属性 (REG_MULTI_SZ)，例如 Hardware ID 和 Compatible ID。
/// 这些属性通常包含多个字符串，每个字符串以 null 结尾，最后再以一个额外的 null 结尾。
fn get_device_property_string_list(
    dev_info: HDEVINFO,
    dev_data: &mut SP_DEVINFO_DATA,
    property: SETUP_DI_REGISTRY_PROPERTY, // SPDRP_HARDWAREID or SPDRP_COMPATIBLEIDS
) -> Vec<String> {
    let mut required_size = 0;
    let mut prop_type = 0;

    unsafe {
        let _ = SetupDiGetDeviceRegistryPropertyW(
            dev_info,
            dev_data,
            property,
            Some(&mut prop_type),
            None,
            Some(&mut required_size),
        );
    }

    if required_size == 0 {
        return Vec::new();
    }

    let mut buffer = vec![0u8; required_size as usize];

    let success = unsafe {
        SetupDiGetDeviceRegistryPropertyW(
            dev_info,
            dev_data,
            property,
            Some(&mut prop_type),
            Some(buffer.as_mut_slice()),
            Some(&mut required_size),
        )
    };

    if success.is_ok() {
        // 转换为 u16 slice
        let wchars: Vec<u16> = buffer
            .chunks_exact(2)
            .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
            .collect();

        // 解析 REG_MULTI_SZ
        // 这种格式是多个以 \0 结尾的字符串连在一起，最后再加一个 \0
        // 例如: "PCI\VEN_xxxx\0PCI\VEN_yyyy\0\0"
        let mut result = Vec::new();
        let mut current_str = Vec::new();

        for &c in &wchars {
            if c == 0 {
                if !current_str.is_empty() {
                    result.push(String::from_utf16_lossy(&current_str));
                    current_str.clear();
                }
            } else {
                current_str.push(c);
            }
        }
        result
    } else {
        Vec::new()
    }
}

/// 检查设备是否缺失驱动
///
/// # 参数
///
/// - `dev_inst`: 设备实例 ID (来自 SP_DEVINFO_DATA.DevInst)
///
/// # 返回值
///
/// - `true`: 设备缺失驱动
/// - `false`: 设备正常运行或根本没启动
///
/// # 说明
///
/// 该函数通过调用 Windows Configuration Manager API 检查设备是否缺失驱动。
/// 它会根据设备的状态和 Problem Code 来判断是否需要安装驱动。
fn is_driver_missing(dev_inst: u32) -> bool {
    let mut status = CM_DEVNODE_STATUS_FLAGS::default();
    let mut problem_number = CM_PROB::default();

    // 调用 Configuration Manager API 获取状态
    let ret = unsafe {
        CM_Get_DevNode_Status(
            &mut status,
            &mut problem_number,
            dev_inst,
            0, // flags
        )
    };

    if ret != CR_SUCCESS {
        // 获取状态失败，保守起见认为不是缺失驱动
        return false;
    }

    // 检查是否有 DN_HAS_PROBLEM 标志
    // 如果没有这个标志，说明设备正常运行（或者根本没启动但也没报错），不需要装驱动
    if !status.contains(DN_HAS_PROBLEM) {
        return false;
    }

    // 细分判断：根据 Problem Code 决定是否属于“缺失驱动”
    match problem_number {
        // 驱动未安装 (最常见的黄叹号)
        CM_PROB_FAILED_INSTALL => true,

        // 设备未配置 (通常也是缺驱动)
        CM_PROB_NOT_CONFIGURED => true,

        // [可选] 需要重新安装
        // 这种情况下重新安装驱动通常能解决问题
        CM_PROB_REINSTALL => true,

        // [排除] 设备被用户手动禁用
        // 这种情况虽然有 Problem，但我们不应该去打扰用户
        CM_PROB_DISABLED => false,

        // [排除] 其他错误 (如 Code 10 启动失败, Code 43 硬件错误)
        // 这些通常不是单纯重装驱动能解决的，或者是已有驱动但不兼容
        // 如果你的策略是只安装“完全没驱动”的设备，这里返回 false
        _ => false,
    }
}
