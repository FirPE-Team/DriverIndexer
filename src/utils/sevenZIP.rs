use crate::utils::utils::write_embed_file;
use crate::TEMP_PATH;
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone)]
pub struct SevenZip {
    zip_program: PathBuf,
}

impl SevenZip {
    pub fn new() -> Result<SevenZip> {
        let zip_program = TEMP_PATH.join("7z.exe");
        if !zip_program.exists() {
            write_embed_file("7z.exe", &zip_program)
                .with_context(|| "Write 7z.exe to temp path failed")?;
            write_embed_file("7z.dll", &TEMP_PATH.join("7z.dll"))
                .with_context(|| "Write 7z.dll to temp path failed")?;
        }
        Ok(SevenZip { zip_program })
    }

    /// 7-zip 创建压缩包
    pub fn create_archive(&self, input_path: &Path, out_path: &Path) -> Result<bool> {
        // 7z a -t7z "文件名.7z" "路径\*" -mx=9 -ms=128m -mmt -r
        let output = Command::new(&self.zip_program)
            .arg("a")
            // 指定7z格式
            .arg("-t7z")
            .arg(out_path)
            .arg(format!("{}\\*", input_path.to_str().unwrap()))
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
    pub fn extract_files(
        &self,
        archive_path: &Path,
        password: Option<&str>,
        extract_path: &str,
        out_path: &Path,
    ) -> Result<bool> {
        let output = Command::new(&self.zip_program)
            .arg("e")
            .arg(archive_path.to_str().unwrap())
            .arg(extract_path)
            .arg("-y")
            .arg("-aos")
            .arg(format!("-p{}", password.unwrap_or("")))
            .arg(format!("-o{}", out_path.to_str().unwrap()))
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
    pub fn extract_files_from_path(
        &self,
        archive_path: &Path,
        password: Option<&str>,
        extract_path: &str,
        out_path: &Path,
    ) -> Result<bool> {
        let output = Command::new(&self.zip_program)
            .arg("x")
            .arg("-r")
            .arg(archive_path.to_str().unwrap())
            .arg(extract_path)
            .arg("-y")
            .arg("-aos")
            .arg(format!("-p{}", password.unwrap_or("")))
            .arg(format!("-o{}", out_path.to_str().unwrap()))
            .output()?;
        let content = String::from_utf8_lossy(&output.stdout);
        Ok(!content.contains("No files to process")
            && !content.contains("Errors")
            && !content.contains("Can't open as archive"))
    }

    /// 判断指定文件是否为驱动包（包含驱动INF文件）
    /// 用于判断自身程序是否为驱动包应用程序
    ///
    /// # 参数
    /// - `path`: 压缩包路径
    ///
    /// # 返回值
    /// - `Result<bool>`: 是否为驱动包
    pub fn is_driver_package(&self, path: &Path) -> Result<bool> {
        let output = Command::new(&self.zip_program)
            .arg("l")
            .arg("-ba")
            .arg("-sccUTF-8")
            .arg(path.to_str().unwrap())
            .output()?;
        let content = String::from_utf8_lossy(&output.stdout);
        Ok(content.to_lowercase().contains(".inf"))
    }
    
    /// 从驱动包中提取索引文件
    /// # 参数
    /// 1. 压缩包路径
    /// 2. 输出路径
    /// # 返回值
    /// 是否成功提取索引文件
    pub fn extract_index(&self, archive_path: &Path, out_path: &Path) -> Result<bool> {
        // 首先列出压缩包内的文件，查找索引文件
        let output = Command::new(&self.zip_program)
            .arg("l")
            .arg("-ba")
            .arg("-sccUTF-8")
            .arg(archive_path.to_str().unwrap())
            .output()?;
        
        let content = String::from_utf8_lossy(&output.stdout);
        
        // 查找索引文件（.index文件）
        for line in content.lines() {
            if line.trim().to_lowercase().ends_with(".index") {
                // 提取文件名
                let parts: Vec<&str> = line.split_whitespace().collect();
                if let Some(filename) = parts.last() {
                    // 提取索引文件
                    return self.extract_files(
                        archive_path,
                        None,
                        filename,
                        out_path
                    );
                }
            }
        }
        
        // 如果找不到索引文件，尝试查找是否有.json文件（可能是索引）
        for line in content.lines() {
            if line.trim().to_lowercase().ends_with(".json") {
                // 提取文件名
                let parts: Vec<&str> = line.split_whitespace().collect();
                if let Some(filename) = parts.last() {
                    // 提取JSON文件
                    return self.extract_files(
                        archive_path,
                        None,
                        filename,
                        out_path
                    );
                }
            }
        }
        
        Ok(false)
    }
}
