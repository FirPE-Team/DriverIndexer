use std::error::Error;
use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use windows::core::{GUID, PCWSTR};
use windows::Win32::Devices::DeviceAndDriverInstallation::{CM_Locate_DevNodeW, CM_Reenumerate_DevNode, SetupDiGetClassDescriptionW, CM_LOCATE_DEVNODE_NORMAL, CONFIGRET};
use windows::Win32::System::Com::CLSIDFromString;

// https://docs.microsoft.com/zh-cn/windows-hardware/drivers/install/using-device-installation-functions
/// 获取硬件信息
/// [参考资料](https://docs.microsoft.com/zh-cn/windows/win32/api/setupapi/nf-setupapi-setupdigetclassdevsexa)
pub unsafe fn getDeviceInfo() {
    // let _hdevInfo: *mut c_void = SetupDiGetClassDevsW(null_mut(), PWSTR::NULL, HWND::NULL, DIGCF_ALLCLASSES);

    // if HANDLE::from(hdevInfo) == INVALID_HANDLE_VALUE {
    //     println!("错误码: {:?}", GetLastError());
    //     return;
    // }
    // println!("{:?}", hdevInfo);
}

/// 扫描检测硬件改动
///
/// 返回
/// - `bool`: 成功返回`true`，失败返回`false`
/// [参考资料](https://www.shuzhiduo.com/A/D854GRg3JE)
pub unsafe fn rescan() -> bool {
    let devInst: *mut u32 = &mut 0;

    let status = CM_Locate_DevNodeW(devInst, None, CM_LOCATE_DEVNODE_NORMAL);
    if status != CONFIGRET(0) {
        return false;
    }

    let status = CM_Reenumerate_DevNode(*devInst, windows::Win32::Devices::DeviceAndDriverInstallation::CM_REENUMERATE_FLAGS(0_u32));
    if status != CONFIGRET(0) {
        return false;
    }
    true
}

/// 获取驱动GUID类说明
///
/// 参数
/// - `guid(GUID)`: GUID类型
///
/// 返回
/// - `Ok(String)`:  成功返回类名的描述
/// - `Err(...)`：   失败则返回错误
pub unsafe fn get_class_description(guid: GUID) -> Result<String, Box<dyn Error>> {
    let mut buf: [u16; 256] = [0; 256];
    let mut needed: u32 = 0;
    SetupDiGetClassDescriptionW(&guid, &mut buf, Some(&mut needed))?;

    let len = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    Ok(String::from_utf16_lossy(&buf[..len]))
}

/// 获取驱动GUID类说明-通过字符串
///
/// 参数
/// - `guid(&str)`: guid 类，需要使用{}包裹guid
///
/// 返回
/// - `Ok(String)`:  成功返回类名的描述
/// - `Err(...)`：   失败则返回错误
pub unsafe fn get_class_description_str(guid_str: &str) -> Result<String, Box<dyn Error>> {
    let guid_wide: Vec<u16> = OsStr::new(guid_str).encode_wide().chain(Some(0)).collect();
    let guid_raw = CLSIDFromString(PCWSTR(guid_wide.as_ptr()))?;

    let mut buf: [u16; 256] = [0; 256];
    let mut needed: u32 = 0;
    SetupDiGetClassDescriptionW(&guid_raw, &mut buf, Some(&mut needed))?;

    let len = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    Ok(String::from_utf16_lossy(&buf[..len]))
}
