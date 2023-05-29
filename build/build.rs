fn main() -> Result<(), Box<dyn std::error::Error>> {
    let build_root = std::env::var("BUILD_ROOT")?;

    for dir in std::fs::read_dir(format!("{}/subprojects", build_root))? {
        let dir = dir?;
        let dir_name = dir.file_name();
        let dir_name = dir_name.as_os_str().to_string_lossy();
        if dir_name.starts_with("nvtop-") {
            println!(
                "cargo:rustc-link-search=native={}/subprojects/{}",
                build_root, dir_name
            );
            println!("cargo:rustc-link-arg=-Wl,--whole-archive");
            println!("cargo:rustc-link-arg=-lnvtop");
            println!("cargo:rustc-link-arg=-Wl,--no-whole-archive");
        }
    }

    println!("cargo:rustc-link-arg=-Wl,-Bdynamic");
    println!("cargo:rustc-link-arg=-lGL");

    Ok(())
}
