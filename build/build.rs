fn main() -> Result<(), Box<dyn std::error::Error>> {
    let build_root =
        std::env::var("BUILD_ROOT").unwrap_or(std::env::var("CARGO_MANIFEST_DIR")? + "/build");

    let subprojects_dir = std::fs::read_dir(format!("{}/subprojects", build_root));
    if subprojects_dir.is_err() {
        return Ok(());
    }

    for dir in subprojects_dir.unwrap() {
        let dir = dir?;
        let dir_name = dir.file_name();
        let dir_name = dir_name.as_os_str().to_string_lossy();
        if dir_name.starts_with("nvtop-") {
            println!(
                "cargo:rustc-link-search=native={}/subprojects/{}",
                build_root, dir_name
            );
            println!("cargo:rustc-link-arg=-lnvtop");

            break;
        }
    }

    println!("cargo:rustc-link-arg=-Wl,-Bdynamic");
    println!("cargo:rustc-link-arg=-lGL");

    Ok(())
}
