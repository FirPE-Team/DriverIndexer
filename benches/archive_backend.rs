//! Same harness is built against main and the bit7z branch. No driver installation.
//! Outputs one JSON record; external orchestration performs interleaving and validation.
use DriverIndexer::temp_workspace::TempWorkspace;
use DriverIndexer::utils::sevenzip::SevenZip;
use anyhow::{Result, anyhow};
use std::path::PathBuf;
use std::time::Instant;

fn main() -> Result<()> {
    let arguments: Vec<_> = std::env::args_os().skip(1).collect();
    if arguments.len() < 3 {
        return Err(anyhow!(
            "usage: archive_backend OP INPUT OUTPUT [ARCHIVE_DIRECTORY]"
        ));
    }
    let operation = arguments[0].to_string_lossy();
    let input = PathBuf::from(&arguments[1]);
    let output = PathBuf::from(&arguments[2]);
    let total = Instant::now();
    let workspace = TempWorkspace::create("archive-benchmark")?;
    let zip = SevenZip::new_in(workspace.path())?;
    let initialized_ms = total.elapsed().as_secs_f64() * 1000.0;
    let started = Instant::now();
    match operation.as_ref() {
        "metadata" => zip.is_driver_package(&input, None)?,
        "inf_cat" => zip.extract_files_from_paths(&input, None, &["*.inf", "*.cat"], &output)?,
        "directory" => {
            let directory = arguments
                .get(3)
                .ok_or_else(|| anyhow!("missing archive directory"))?
                .to_string_lossy();
            zip.extract_files_from_path(&input, None, &directory, &output)?;
        }
        "full" => zip.extract_files_from_path(&input, None, "*", &output)?,
        "compress" => zip.create_archive(&input, &output)?,
        _ => return Err(anyhow!("unknown operation: {operation}")),
    }
    let operation_ms = started.elapsed().as_secs_f64() * 1000.0;
    drop(zip);
    workspace.cleanup()?;
    println!(
        "{}",
        serde_json::json!({
            "operation_ms": operation_ms,
            "initialization_ms": initialized_ms,
            "total_ms": total.elapsed().as_secs_f64() * 1000.0,
        })
    );
    Ok(())
}
