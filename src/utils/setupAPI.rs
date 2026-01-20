use anyhow::{anyhow, Context, Result};
use std::ffi::c_void;
use std::path::Path;
use windows::core::{GUID, HSTRING, PCWSTR};
use windows::Win32::Devices::DeviceAndDriverInstallation::{
    CM_Locate_DevNodeW, CM_Reenumerate_DevNode, SetupCloseInfFile, SetupDiClassGuidsFromNameExW,
    SetupDiGetClassDescriptionW, SetupFindFirstLineW, SetupFindNextLine, SetupGetFieldCount,
    SetupGetStringFieldW, SetupOpenInfFileW, CM_LOCATE_DEVNODE_NORMAL, CONFIGRET,
    INFCONTEXT, INF_STYLE_WIN4,
};
use windows::Win32::Foundation::{GetLastError, INVALID_HANDLE_VALUE};
use windows::Win32::System::Com::CLSIDFromString;

/// 封装Windows SetupAPI函数
///
/// # 参考
/// [Windows SetupAPI文档](https://learn.microsoft.com/zh-cn/windows/win32/api/setupapi/)
pub struct SetupAPI {}

impl SetupAPI {
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

    /// 根据类名获取对应的GUID列表
    ///
    /// # 参数
    /// - `class_name(&str)`: 设备类名称
    ///
    /// # 返回值
    /// - `Ok(Vec<GUID>)`: 成功返回GUID列表
    /// - `Err(e)`: 失败则返回错误信息
    pub fn get_class_guids_from_name(class_name: &str) -> Result<Vec<GUID>> {
        // 将类名转换为宽字符串
        let class_name_wide = HSTRING::from(class_name);

        // 先尝试使用一个小的缓冲区
        let mut guid_list = vec![GUID::default(); 1];
        let mut returned_count: u32 = 0;

        // 第一次调用，使用小缓冲区获取实际需要的GUID数量
        let result = unsafe {
            SetupDiClassGuidsFromNameExW(
                PCWSTR(class_name_wide.as_ptr()),
                &mut guid_list,
                &mut returned_count,
                None,
                None,
            )
        };

        // 如果缓冲区太小，returned_count会包含实际需要的数量
        if let Err(e) = result {
            // 检查错误码是否为缓冲区太小 (ERROR_INSUFFICIENT_BUFFER)
            if e.code().0 == -2147024774i32 {
                // 0x8007007A as i32
                // 使用返回的数量重新分配缓冲区
                guid_list = vec![GUID::default(); returned_count as usize];

                // 再次调用，这次应该成功
                unsafe {
                    SetupDiClassGuidsFromNameExW(
                        PCWSTR(class_name_wide.as_ptr()),
                        &mut guid_list,
                        &mut returned_count,
                        None,
                        None,
                    )
                }
                .with_context(|| "Failed to get GUID list")?;
            } else {
                return Err(e).with_context(|| "Failed to get required GUID count");
            }
        }

        // 调整返回的GUID数量
        guid_list.truncate(returned_count as usize);
        Ok(guid_list)
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

    /// 根据字符串获取GUID
    ///
    /// # 参数
    /// - `guid_str(&str)`: guid 类，需要使用{}包裹guid字符串
    ///
    /// # 返回值
    /// - `Ok(GUID)`: 成功返回GUID
    /// - `Err(e)`: 失败则返回错误信息
    pub fn get_guid_from_str(guid_str: &str) -> Result<GUID> {
        let guid_hstring = HSTRING::from(guid_str);
        let guid = unsafe { CLSIDFromString(PCWSTR(guid_hstring.as_ptr())) }
            .with_context(|| "CLSIDFromString failed")?;
        Ok(guid)
    }

    /// 打开inf文件
    ///
    /// # 参数
    /// - `inf_path(&str)`: inf 文件路径
    ///
    /// # 返回值
    /// - `*mut c_void`: 成功返回inf文件句柄，失败返回`null_mut()`
    pub fn open_inf_file(inf_path: &Path) -> Result<*mut c_void> {
        let inf_hstring = HSTRING::from(inf_path);
        unsafe {
            let handle =
                SetupOpenInfFileW(PCWSTR(inf_hstring.as_ptr()), None, INF_STYLE_WIN4, None);
            if handle == INVALID_HANDLE_VALUE.0 {
                return Err(anyhow!("SetupOpenInfFileW failed: {:?}", GetLastError()));
            }

            Ok(handle)
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
        key: Option<&str>,
    ) -> Result<INFCONTEXT> {
        // 将section转换为宽字符串
        let section_wide = HSTRING::from(section);

        // 将key转换为宽字符串
        let key_wide = match key {
            Some(key) => {
                let key_hstring = HSTRING::from(key);
                PCWSTR(key_hstring.as_ptr())
            }
            None => PCWSTR::null(),
        };

        let mut context = INFCONTEXT::default();
        unsafe {
            SetupFindFirstLineW(
                inf_handle,
                PCWSTR(section_wide.as_ptr()),
                key_wide,
                &mut context,
            )?;
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
            SetupGetStringFieldW(context, field_index, Some(&mut buf), Some(&mut needed))?;
        }
        let len = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
        Ok(String::from_utf16_lossy(&buf[..len]))
    }

    /// 查找inf文件中的下一行
    ///
    /// # 参数
    /// - `context(&mut INFCONTEXT)`: inf 文件上下文
    ///
    /// # 返回值
    /// - `Ok(INFCONTEXT)`: 成功返回INFCONTEXT结构体，失败返回错误信息
    pub fn find_next_line(context: &mut INFCONTEXT) -> Result<INFCONTEXT> {
        let mut result = INFCONTEXT::default();
        unsafe {
            SetupFindNextLine(context, &mut result)?;
        }
        Ok(result)
    }
}
