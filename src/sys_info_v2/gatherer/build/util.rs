pub fn find_program(name: &str) -> Option<String> {
    #[cfg(windows)]
    const PATH_VAR_SEPARATOR: char = ';';

    #[cfg(not(windows))]
    const PATH_VAR_SEPARATOR: char = ':';

    let path = if let Ok(path) = std::env::var("PATH") {
        path
    } else {
        "".into()
    };
    for path in path.split(PATH_VAR_SEPARATOR) {
        let program_path = format!("{}/{}", path, name);
        if std::path::Path::new(&program_path).exists() {
            return Some(program_path);
        }
    }

    None
}

pub fn download_file(
    url: &str,
    path: &str,
    checksum: Option<&str>,
) -> Result<String, Box<dyn std::error::Error>> {
    use std::io::Write;

    std::fs::create_dir_all(path)?;

    let file_name = url.split('/').last().unwrap();
    let output_path = format!("{}/{}", path, file_name);

    if std::path::Path::new(&output_path).exists() {
        return Ok(output_path);
    }

    let response = ureq::get(url).call()?;

    if response.status() != 200 {
        return Err(format!(
            "Failed to download {}. HTTP status code {} {}",
            url,
            response.status(),
            response.status_text()
        )
        .into());
    }

    let mut content = Vec::new();
    response.into_reader().read_to_end(&mut content)?;

    let mut sha256 = cargo_util::Sha256::new();
    sha256.update(&content);

    if let Some(expected_checksum) = checksum {
        let actual_checksum = sha256.finish_hex();
        if actual_checksum != expected_checksum {
            return Err(format!(
                "Checksum validation failed! Expected: {} Actual: {}",
                expected_checksum, actual_checksum,
            )
            .into());
        }
    }

    let mut file = std::fs::File::create(&output_path)?;
    file.write_all(content.as_slice())?;
    file.flush()?;

    println!("cargo:rerun-if-changed={}", output_path);

    Ok(output_path)
}
