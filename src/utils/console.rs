use crate::utils::utils::write_log;
use crate::LOG_PATH;
use console::style;
use rust_i18n::t;

pub enum ConsoleType {
    /// 普通信息
    Info,
    /// 成功信息
    Success,
    /// 警告信息
    Warning,
    /// 错误信息
    Error,
    /// 调试信息
    Debug,
}

/// 写入控制台
///
/// # 参数
/// - `consoleType`: 控制台类型
/// - `message`: 控制台消息
///
/// # 返回值
/// - `Ok(())`: 写入成功
pub fn write_console(consoleType: ConsoleType, message: &str) {
    let title = match &consoleType {
        ConsoleType::Info => style(t!("console.info")).cyan(),
        ConsoleType::Success => style(t!("console.success")).green(),
        ConsoleType::Warning => style(t!("console.warning")).yellow(),
        ConsoleType::Error => style(t!("console.err")).red().on_black().bold(),
        ConsoleType::Debug => style(t!("console.debug")).blue(),
    };
    println!("  {}      {}", &title, message);

    // 写入日志文件
    if let Some(log_path) = LOG_PATH.get() {
        write_log(
            log_path,
            &format!(
                "{}  {}",
                console::strip_ansi_codes(&title.to_string()),
                message
            ),
        )
        .ok();
    }
}
