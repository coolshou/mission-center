fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rustc-link-arg=-Wl,-Bdynamic");
    println!("cargo:rustc-link-arg=-lGL");

    Ok(())
}
