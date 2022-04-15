use std::{env, fs, path::PathBuf};

use parcel_css::stylesheet::{ParserOptions, PrinterOptions, StyleSheet};
use sha2::{Digest, Sha256};

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
                    filename,
                    file_contents.as_str(),
                    ParserOptions {
                        css_modules: false,
                        custom_media: false,
                        nesting: true,
                        source_index: 0,
                    },
                )
                .unwrap();

                let minified_stylesheet = stylesheet
                    .to_css(PrinterOptions {
                        analyze_dependencies: false,
                        minify: true,
                        pseudo_classes: None,
                        source_map: None,
                        targets: None,
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
                    format!("'sha256-{}'", base64::encode(hash.as_slice())),
                )
                .unwrap();
            }
        }
    }

    println!("cargo:rerun-if-changed=build.rs");
    println!(
        "cargo:rerun-if-changed={}",
        asset_dir.as_os_str().to_str().unwrap()
    );
}
