use anyhow::{Context, Result};
use std::ffi::{c_void, OsStr};
use std::os::windows::ffi::OsStrExt;
use std::path::Path;
use windows::core::{GUID, PCWSTR};
use windows::Win32::Devices::DeviceAndDriverInstallation::{
    CM_Locate_DevNodeW, CM_Reenumerate_DevNode, SetupCloseInfFile, SetupDiGetClassDescriptionW,
    SetupFindFirstLineW, SetupFindNextLine, SetupGetFieldCount, SetupGetStringFieldW, SetupOpenInfFileW,
    CM_LOCATE_DEVNODE_NORMAL, CONFIGRET, INFCONTEXT, INF_STYLE_OLDNT,
    INF_STYLE_WIN4,
};
use windows::Win32::System::Com::CLSIDFromString;

/// 封装Windows SetupAPI函数
///
/// # 参考
/// [Windows SetupAPI文档](https://learn.microsoft.com/zh-cn/windows/win32/api/setupapi/)
pub struct SetupAPI {}

impl SetupAPI {
    // https://docs.microsoft.com/zh-cn/windows-hardware/drivers/install/using-device-installation-functions
    /// 获取硬件信息
    /// [参考资料](https://docs.microsoft.com/zh-cn/windows/win32/api/setupapi/nf-setupapi-setupdigetclassdevsexa)
    pub fn _get_device_info() {
        // let _hdevInfo: *mut c_void = SetupDiGetClassDevsW(null_mut(), PWSTR::NULL, HWND::NULL, DIGCF_ALLCLASSES);

        // if HANDLE::from(hdevInfo) == INVALID_HANDLE_VALUE {
        //     println!("错误码: {:?}", GetLastError());
        //     return;
        // }
        // println!("{:?}", hdevInfo);
    }

    /// 扫描检测硬件改动 [参考资料](https://www.shuzhiduo.com/A/D854GRg3JE)
    ///
    /// # 返回值
    /// - `bool`: 成功返回`true`，失败返回`false`
    pub fn rescan() -> bool {
        let devInst: *mut u32 = &mut 0;

        let status = unsafe { CM_Locate_DevNodeW(devInst, None, CM_LOCATE_DEVNODE_NORMAL) };
        if status != CONFIGRET(0) {
            return false;
        }

        let status = unsafe {
            CM_Reenumerate_DevNode(
                *devInst,
                windows::Win32::Devices::DeviceAndDriverInstallation::CM_REENUMERATE_FLAGS(0_u32),
            )
        };
        if status != CONFIGRET(0) {
            return false;
        }

        true
    }

    /// 获取驱动GUID类说明
    ///
    /// # 参数
    /// - `guid(GUID)`: GUID类型
    ///
    /// # 返回值
    /// - `Ok(String)`: 成功返回类名的描述
    /// - `Err(e)`: 失败则返回错误信息
    pub fn get_class_description_from_guid(guid: &GUID) -> Result<String> {
        let mut buf: [u16; 256] = [0; 256];
        let mut needed: u32 = 0;
        unsafe {
            SetupDiGetClassDescriptionW(guid, &mut buf, Some(&mut needed))
                .with_context(|| "Get driver class description failed")?;
        }
        let len = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
        Ok(String::from_utf16_lossy(&buf[..len]))
    }

    /// 获取驱动GUID类说明-通过字符串
    ///
    /// # 参数
    /// - `guid_str(&str)`: guid 类，需要使用{}包裹guid字符串
    ///
    /// # 返回值
    /// - `Ok(String)`: 成功返回类名的描述
    /// - `Err(e)`: 失败则返回错误信息
    pub fn get_class_description_from_str(guid_str: &str) -> Result<String> {
        let guid_wide: Vec<u16> = OsStr::new(guid_str).encode_wide().chain(Some(0)).collect();
        let guid_raw = unsafe { CLSIDFromString(PCWSTR(guid_wide.as_ptr())) }
            .with_context(|| "CLSIDFromString failed")?;

        let mut buf: [u16; 256] = [0; 256];
        let mut needed: u32 = 0;
        unsafe {
            SetupDiGetClassDescriptionW(&guid_raw, &mut buf, Some(&mut needed))
                .with_context(|| "Get driver class description failed")?;
        }

        let len = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
        Ok(String::from_utf16_lossy(&buf[..len]))
    }

    /// 打开inf文件
    ///
    /// # 参数
    /// - `inf_path(&str)`: inf 文件路径
    ///
    /// # 返回值
    /// - `*mut c_void`: 成功返回inf文件句柄，失败返回`null_mut()`
    pub fn open_inf_file(inf_path: &Path) -> *mut c_void {
        let inf_wide: Vec<u16> = inf_path.as_os_str().encode_wide().chain(Some(0)).collect();
        unsafe {
            SetupOpenInfFileW(
                PCWSTR(inf_wide.as_ptr()),
                None,
                INF_STYLE_OLDNT | INF_STYLE_WIN4,
                None,
            )
        }
    }

    /// 关闭inf文件
    ///
    /// # 参数
    /// - `inf_handle(*mut c_void)`: inf 文件句柄
    pub fn close_inf_file(inf_handle: *mut c_void) {
        unsafe {
            SetupCloseInfFile(inf_handle);
        }
    }

    /// 查找inf文件中的第一行
    ///
    /// # 参数
    /// - `inf_handle(*mut c_void)`: inf 文件句柄
    /// - `section(&str)`: 要查找的节(section)
    /// - `key(&str)`: 要查找的键(key)
    ///
    /// # 返回值
    /// - `Ok(INFCONTEXT)`: 成功返回INFCONTEXT结构体，失败返回错误信息
    pub fn find_first_line(
        inf_handle: *mut c_void,
        section: &str,
        key: &str,
    ) -> Result<INFCONTEXT> {
        let section_wide: Vec<u16> = OsStr::new(section).encode_wide().chain(Some(0)).collect();
        let key_wide: Vec<u16> = OsStr::new(key).encode_wide().chain(Some(0)).collect();
        let mut context = INFCONTEXT::default();
        unsafe {
            SetupFindFirstLineW(
                inf_handle,
                PCWSTR(section_wide.as_ptr()),
                PCWSTR(key_wide.as_ptr()),
                &mut context,
            )
            .with_context(|| "SetupFindFirstLineW failed")?;
        }
        Ok(context)
    }

    /// 获取inf文件中指定节(section)的字段数量
    ///
    /// # 参数
    /// - `inf_handle(*mut c_void)`: inf 文件句柄
    /// - `section(&str)`: 要查找的节(section)
    ///
    /// # 返回值
    /// - `Ok(u32)`: 成功返回字段数量，失败返回错误信息
    pub fn get_field_count(context: &INFCONTEXT) -> u32 {
        unsafe { SetupGetFieldCount(context) }
    }

    /// 获取inf文件中指定字段的值
    ///
    /// # 参数
    /// - `context(&INFCONTEXT)`: inf 文件上下文
    /// - `field_index(u32)`: 要获取的字段索引
    ///
    /// # 返回值
    /// - `Ok(String)`: 成功返回字段值，失败返回错误信息
    pub fn get_string_field(context: &INFCONTEXT, field_index: u32) -> Result<String> {
        let mut buf: [u16; 256] = [0; 256];
        let mut needed: u32 = 0;
        unsafe {
            SetupGetStringFieldW(context, field_index, Some(&mut buf), Some(&mut needed))
                .with_context(|| "SetupGetStringFieldW failed")?;
        }
        let len = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
        Ok(String::from_utf16_lossy(&buf[..len]))
    }

    pub fn find_next_line(context: &mut INFCONTEXT) -> Result<INFCONTEXT> {
        let mut result = INFCONTEXT::default();
        unsafe {
            SetupFindNextLine(context, &mut result).with_context(|| "SetupFindNextLine failed")?;
        }
        Ok(result)
    }
}
