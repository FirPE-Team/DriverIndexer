use std::error::Error;
use std::path::{Path, PathBuf};
use std::process::Command;
use crate::utils::util::writeEmbedFile;
use crate::TEMP_PATH;

pub struct sevenZip {
    zipProgram: PathBuf,
}

impl sevenZip {
    pub fn new() -> Result<sevenZip, Box<dyn Error>> {
        let zipProgram = TEMP_PATH.join("7z.exe");
        writeEmbedFile("7z.exe", &zipProgram)?;
        writeEmbedFile("7z.dll", &TEMP_PATH.join("7z.dll"))?;
        Ok(sevenZip { zipProgram })
    }

    /// 7-zip 创建压缩包
    pub fn createArchivePage(
        &self,
        inputPath: &Path,
        outPath: &Path,
    ) -> Result<bool, Box<dyn Error>> {
        // 7z a -t7z "文件名.7z" "路径\*" -mx=9 -ms=128m -mmt -r
        let output = Command::new(&self.zipProgram)
            .arg("a")
            // 指定7z格式
            .arg("-t7z")
            .arg(outPath)
            .arg(format!("{}\\*", inputPath.to_str().unwrap()))
            // 极限压缩
            .arg("-mx=9")
            // 固实压缩（8MB分块）
            .arg("-ms=8m")
            // 启用多线程
            .arg("-mmt")
            // 递归子目录
            .arg("-r")
            .output()?;
        let content = String::from_utf8_lossy(&output.stdout);
        Ok(content.contains("Everything is Ok"))
    }

    /// 7-zip 释放文件（指定压缩包内文件）
    /// 从存档中提取文件（不使用目录名）
    /// 注意：此命令会将压缩档案中的所有文件输出到同一个目录中
    /// # 参数
    /// 1. 压缩包路径
    /// 2. 压缩包密码
    /// 3. 压缩包内文件路径
    /// 4. 输出路径
    pub fn extractFiles(
        &self,
        zipFile: &Path,
        password: Option<&str>,
        extractPath: &str,
        outPath: &Path,
    ) -> Result<bool, Box<dyn Error>> {
        let output = Command::new(&self.zipProgram)
            .arg("e")
            .arg(zipFile.to_str().unwrap())
            .arg(extractPath)
            .arg("-y")
            .arg("-aos")
            .arg(format!("-p{}", password.unwrap_or("")))
            .arg(format!("-o{}", outPath.to_str().unwrap()))
            .output()?;
        let content = String::from_utf8_lossy(&output.stdout);
        Ok(!content.contains("No files to process"))
    }

    /// 7-zip 解压文件
    /// 提取具有完整路径的文件（递归子目录）
    /// 可用于解压指定文件（inf）
    /// # 参数
    /// 1. 压缩包路径
    /// 2. 压缩包密码
    /// 3. 压缩包内文件路径（解压全部文件为*）
    /// 4. 输出路径
    pub fn extractFilesFromPath(
        &self,
        zipFile: &Path,
        password: Option<&str>,
        extractPath: &str,
        outPath: &Path,
    ) -> Result<bool, Box<dyn Error>> {
        let output = Command::new(&self.zipProgram)
            .arg("x")
            .arg("-r")
            .arg(zipFile.to_str().unwrap())
            .arg(extractPath)
            .arg("-y")
            .arg("-aos")
            .arg(format!("-p{}", password.unwrap_or("")))
            .arg(format!("-o{}", outPath.to_str().unwrap()))
            .output()?;
        let content = String::from_utf8_lossy(&output.stdout);
        Ok(!content.contains("No files to process") && !content.contains("Errors") && !content.contains("Can't open as archive"))
    }

    /// 判断指定文件是否为驱动包（包含驱动INF文件）
    /// 用于判断自身程序是否为驱动包应用程序
    /// # 参数
    /// 1. 压缩包路径
    pub fn isDriverPackage(&self, zipFile: &Path) -> Result<bool, Box<dyn Error>> {
        let output = Command::new(&self.zipProgram)
            .arg("l")
            .arg("-ba")
            .arg("-sccUTF-8")
            .arg(zipFile.to_str().unwrap())
            .output()?;
        let content = String::from_utf8_lossy(&output.stdout);
        Ok(content.to_lowercase().contains(".inf"))
    }
}
