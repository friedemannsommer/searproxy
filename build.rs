use std::{env, fs, path::PathBuf};

use base64::Engine;
use lightningcss::stylesheet::{ParserOptions, PrinterOptions, StyleSheet};
use sha2::{Digest, Sha256};

const BASE64_ENGINE: base64::engine::GeneralPurpose = base64::engine::GeneralPurpose::new(
    &base64::alphabet::STANDARD,
    base64::engine::GeneralPurposeConfig::new()
        .with_encode_padding(false)
        .with_decode_padding_mode(base64::engine::DecodePaddingMode::Indifferent),
);

fn main() {
    let out_dir = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    let package_dir = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap());
    let asset_dir = package_dir.join("./src/assets");

    for dir_entry in fs::read_dir(&asset_dir).unwrap().flatten() {
        let path = dir_entry.path();

        if let Some(extension) = path.extension() {
            if extension == "css" {
                let filename = path.file_name().unwrap().to_str().unwrap();
                let file_contents = String::from_utf8(fs::read(&path).unwrap()).unwrap();
                let stylesheet = StyleSheet::parse(
                    file_contents.as_str(),
                    ParserOptions {
                        custom_media: false,
                        error_recovery: false,
                        filename: filename.to_string(),
                        nesting: true,
                        ..ParserOptions::default()
                    },
                )
                .unwrap();

                let minified_stylesheet = stylesheet
                    .to_css(PrinterOptions {
                        minify: true,
                        ..Default::default()
                    })
                    .unwrap();
                let mut hasher = Sha256::new();

                hasher.update(&minified_stylesheet.code);

                let hash = hasher.finalize();
                let mut filename_path = PathBuf::from(filename);

                fs::write(out_dir.join(&filename_path), minified_stylesheet.code).unwrap();
                filename_path.set_extension("hash");
                fs::write(
                    out_dir.join(filename_path),
                    format!("'sha256-{}'", BASE64_ENGINE.encode(hash.as_slice())),
                )
                .unwrap();
            }
        }
    }

    println!(
        "cargo:rerun-if-changed={}",
        asset_dir.as_os_str().to_str().unwrap()
    );
}
