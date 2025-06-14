use crate::utils::util::{extract_vars, getFileList, String_utils};
use chardet::{charset2encoding, detect};
use encoding::label::encoding_from_whatwg_label;
use encoding::DecoderTrap;
use std::error::Error;
use std::fs;
use std::fs::File;
use std::io::Read;
use std::path::Path;

pub fn classify_driver(driverPath: &Path) -> Result<(), Box<dyn Error>> {
    // 遍历INF文件
    let infList = getFileList(driverPath, "*.inf")?;

    // 解析INF文件
    for infPath in infList.iter() {
        // 目标路径
        let mut resultPath = driverPath.to_path_buf();

        // 读取INF文件
        let mut file = File::open(infPath)?;
        let mut fileBuf: Vec<u8> = Vec::new();
        file.read_to_end(&mut fileBuf)?;

        // 自动识别编码并以UTF-8读取
        let result = detect(&fileBuf);
        let coder = encoding_from_whatwg_label(charset2encoding(&result.0)).ok_or("Failed encoding")?;
        let infContent = coder.decode(&fileBuf, DecoderTrap::Ignore)?;

        // 去除INF内的所有 空格 与 tab符
        let infContent = infContent.replace(" ", "").replace("	", "");

        let mut class = "UnknownClass";
        // let mut provider = "UnknownProvider".to_string();
        let mut provider = String::from("UnknownProvider");

        // 按行读取
        for line in infContent.lines() {
            // 空行、行首注释
            if line.is_empty() || line.starts_with(";") { continue; }
            // 行尾注释
            let line = line.split(';').next().unwrap_or(line).trim();

            // 驱动类别
            if let Some(v) = line.strip_prefix("Class=") {
                class = v;
            }

            // if let Some(v) = line.strip_prefix("Provider=") {
            //     // 变量替换处理
            //     provider = extract_vars(v).iter().fold(
            //         String::from(&*v.to_string()),
            //         |acc, provider| {
            //             String::from(&*infContent.get_string_center(&format!("{provider}="), "\r\n")
            //                 .map(|v| acc.replace(&format!("%{provider}%"), v.trim_matches('"')))
            //                 .unwrap_or(acc.to_string()))
            //         },
            //     );
            // }


            if let Some(v) = line.strip_prefix("Provider=") {
                // 变量替换处理
                provider = extract_vars(v).iter().fold(
                    String::from(v),
                    |acc, provider| {
                        infContent.get_string_center(&format!("{provider}="), "\r\n")
                            .map(|v| acc.replace(&format!("%{provider}%"), v.trim_matches('"')))
                            .unwrap_or(acc)
                    },
                );
            }
        }

        // 构建目标路径：driver_path / Class / Provider / <原始包目录名>
        let pkg_root = infPath.parent().unwrap();
        let pkg_name = pkg_root.file_name().unwrap();
        let target = driverPath.join(class).join(provider).join(pkg_name);

        fs::create_dir_all(&resultPath)?;
        println!("Moved `{:?}` => `{:?}`", pkg_root, target);
        // fs::rename(pkg_root, target)?;
    }
    Ok(())
}
