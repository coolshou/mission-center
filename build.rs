fn main() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(target_os = "linux")]
    {
        println!("cargo:rustc-link-arg=-Wl,-Bdynamic");
        println!("cargo:rustc-link-arg=-lGL");
    }

    prost_build::Config::new()
        .compile_protos(
            &[
                "subprojects/magpie/platform/src/proto/apps.proto",
                "subprojects/magpie/platform/src/proto/common.proto",
                "subprojects/magpie/platform/src/proto/ipc.proto",
                "subprojects/magpie/platform/src/proto/processes.proto",
            ],
            &["subprojects/magpie/platform/src/proto/"],
        )
        .unwrap();

    Ok(())
}
