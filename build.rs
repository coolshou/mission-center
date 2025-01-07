fn main() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(target_os = "linux")]
    {
        println!("cargo:rustc-link-arg=-Wl,-Bdynamic");
        println!("cargo:rustc-link-arg=-lGL");
    }

    prost_build::Config::new()
        .compile_protos(
            &[
                "src/proto/apps.proto",
                "src/proto/common.proto",
                "src/proto/ipc.proto",
                "src/proto/processes.proto",
            ],
            &["src/proto/"],
        )
        .unwrap();

    Ok(())
}
