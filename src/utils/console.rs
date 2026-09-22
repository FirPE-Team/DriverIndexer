use crate::utils::utils::write_log;
use console::style;
use rust_i18n::t;
use std::cmp::PartialEq;

#[derive(PartialEq)]
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

    if consoleType == ConsoleType::Error {
        eprintln!("  {}      {}", title, message);
    } else {
        println!("  {}      {}", title, message);
    }

    let level = match consoleType {
        ConsoleType::Info => "INFO",
        ConsoleType::Success => "SUCCESS",
        ConsoleType::Warning => "WARNING",
        ConsoleType::Error => "ERROR",
        ConsoleType::Debug => "DEBUG",
    };
    let _ = write_log(level, message);
}

/// 输出普通文本，并在启用日志时记录为 OUTPUT。
pub fn write_plain(message: &str) {
    println!("{message}");
    let _ = write_log("OUTPUT", message);
}
