#![allow(non_snake_case)]
#![allow(non_camel_case_types)]
#![allow(dead_code)]

extern crate dotenvy_macro;
#[macro_use]
extern crate lazy_static;

rust_i18n::i18n!("locales");

pub mod driver_index;
pub mod driver_match;
pub mod hardware;
pub mod utils;

use dotenvy_macro::dotenv;
use rust_embed::Embed;
use std::env::temp_dir;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;

use crate::utils::utils::get_temp_name;

#[cfg(target_arch = "x86_64")]
#[derive(Embed)]
#[folder = "./assets-x64"]
pub struct Asset;

#[cfg(target_arch = "x86")]
#[derive(Embed)]
#[folder = "./assets-x86"]
pub struct Asset;

#[cfg(target_arch = "aarch64")]
#[derive(Embed)]
#[folder = "./assets-ARM64"]
pub struct Asset;

pub static SECRET_KEY: &str = dotenv!("SECRET_KEY");
pub static DEBUG: AtomicBool = AtomicBool::new(false);

lazy_static! {
    pub static ref TEMP_PATH: PathBuf = temp_dir().join(get_temp_name(".tmp", "", 6));
}
