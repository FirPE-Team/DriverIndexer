use std::os::windows::ffi::OsStrExt;
use std::path::Path;
use windows::core::PCWSTR;
use windows::{
    Win32::Devices::DeviceAndDriverInstallation::UpdateDriverForPlugAndPlayDevicesW,
    Win32::Devices::DeviceAndDriverInstallation::UPDATEDRIVERFORPLUGANDPLAYDEVICES_FLAGS,
    Win32::Foundation::{BOOL, HWND},
};

/// 更新驱动
/// [相关文档](https://docs.microsoft.com/zh-cn/windows/win32/api/newdev/nf-newdev-updatedriverforplugandplaydevicesa?redirectedfrom=MSDN)
/// # 参数
/// 1. INF路径
/// 2. 硬件ID
pub unsafe fn updateDriverForPlugAndPlayDevices(infPath: &Path, hwId: &str) -> bool {
    let infPath: Vec<u16> = infPath.as_os_str().encode_wide().chain(Some(0)).collect();
    let hwId: Vec<u16> = hwId.encode_utf16().chain(Some(0)).collect();
    let mut isReboot = BOOL(0);
    UpdateDriverForPlugAndPlayDevicesW(Some(HWND(std::ptr::null_mut())), PCWSTR::from_raw(hwId.as_ptr()), PCWSTR::from_raw(infPath.as_ptr()), UPDATEDRIVERFORPLUGANDPLAYDEVICES_FLAGS(0), Some(&mut isReboot))
        .is_ok()
}
