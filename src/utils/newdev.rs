use std::os::windows::ffi::OsStrExt;
use std::path::Path;
use windows::core::{BOOL, PCWSTR};
use windows::Win32::Devices::DeviceAndDriverInstallation::UpdateDriverForPlugAndPlayDevicesW;
use windows::Win32::Devices::DeviceAndDriverInstallation::{
    INSTALLFLAG_FORCE, UPDATEDRIVERFORPLUGANDPLAYDEVICES_FLAGS,
};

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
) -> Result<(), windows::core::Error> {
    let hardware_id_w: Vec<u16> = hardware_id.encode_utf16().chain(Some(0)).collect();
    let inf_path_w: Vec<u16> = inf_path.as_os_str().encode_wide().chain(Some(0)).collect();

    let mut isReboot = BOOL(0);
    unsafe {
        UpdateDriverForPlugAndPlayDevicesW(
            None,
            PCWSTR::from_raw(hardware_id_w.as_ptr()),
            PCWSTR::from_raw(inf_path_w.as_ptr()),
            if force {
                INSTALLFLAG_FORCE
            } else {
                UPDATEDRIVERFORPLUGANDPLAYDEVICES_FLAGS(0)
            },
            Some(&mut isReboot),
        )
    }
}
