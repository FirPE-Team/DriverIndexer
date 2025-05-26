fn main() {
    // 兼容Windows7、WindowsXP
    #[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
    thunk::thunk();

    // 内置资源
    embed_resource::compile("./resource/resource.rc", embed_resource::NONE);
}
