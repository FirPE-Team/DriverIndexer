extern crate embed_resource;

fn main() {
    // 版本信息
    embed_resource::compile("./resource/resource.rc", embed_resource::NONE)
        .manifest_optional()
        .expect("Add resource error");

    // 兼容 Windows 7、Windows XP
    #[cfg(not(debug_assertions))]
    #[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
    thunk::thunk();
}
