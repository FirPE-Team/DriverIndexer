@echo off
cd /d %~dp0

rem X64
cargo build --release
IF EXIST upx.exe "%cd%\target\release\DriverIndexer.exe" --best --compress-resources=0 --strip-relocs=0 --compress-icons=0 --compress-exports=0 --lzma

rem X86
cargo build --release --target=i686-pc-windows-msvc
IF EXIST upx.exe "%cd%\target\i686-pc-windows-msvc\release\DriverIndexer.exe" --best --compress-resources=0 --strip-relocs=0 --compress-icons=0 --compress-exports=0 --lzma

rem ARM64
cargo build --release --target=aarch64-pc-windows-msvc
