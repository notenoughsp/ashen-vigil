// Gives the Windows executable its icon: draws it with src/icon_art.rs, writes a .res
// resource file by hand and hands it to the MSVC linker. No extra crates needed.

use std::path::PathBuf;

include!("src/icon_art.rs");

const SIZES: [usize; 6] = [16, 24, 32, 48, 64, 256];

/// One icon image as Windows stores it: a 32-bit DIB, bottom-up, with an empty AND mask.
fn dib(size: usize) -> Vec<u8> {
    let rgba = icon_rgba(size);
    let mut out = Vec::new();
    let mask_row = size.div_ceil(32) * 4;
    out.extend_from_slice(&40u32.to_le_bytes());
    out.extend_from_slice(&(size as i32).to_le_bytes());
    // The height counts the colour bitmap and the mask.
    out.extend_from_slice(&((size * 2) as i32).to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&32u16.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(&((size * size * 4 + mask_row * size) as u32).to_le_bytes());
    out.extend_from_slice(&[0u8; 16]);
    for y in (0..size).rev() {
        for x in 0..size {
            let i = (y * size + x) * 4;
            out.extend_from_slice(&[rgba[i + 2], rgba[i + 1], rgba[i], rgba[i + 3]]);
        }
    }
    out.extend(std::iter::repeat_n(0u8, mask_row * size));
    out
}

/// One entry of a .res file with a numeric type and name.
fn resource(out: &mut Vec<u8>, kind: u16, id: u16, flags: u16, data: &[u8]) {
    out.extend_from_slice(&(data.len() as u32).to_le_bytes());
    out.extend_from_slice(&32u32.to_le_bytes());
    out.extend_from_slice(&[0xFF, 0xFF]);
    out.extend_from_slice(&kind.to_le_bytes());
    out.extend_from_slice(&[0xFF, 0xFF]);
    out.extend_from_slice(&id.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(&flags.to_le_bytes());
    // English (United States).
    out.extend_from_slice(&0x0409u16.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(data);
    while !out.len().is_multiple_of(4) {
        out.push(0);
    }
}

fn main() {
    println!("cargo:rerun-if-changed=src/icon_art.rs");
    println!("cargo:rerun-if-changed=build.rs");
    let target = std::env::var("TARGET").unwrap_or_default();
    if !target.contains("windows-msvc") {
        return;
    }
    let images: Vec<Vec<u8>> = SIZES.iter().map(|s| dib(*s)).collect();
    let mut res = Vec::new();
    // A .res file starts with an empty entry.
    resource(&mut res, 0, 0, 0, &[]);
    for (i, img) in images.iter().enumerate() {
        resource(&mut res, 3, i as u16 + 1, 0x1010, img);
    }
    let mut group = Vec::new();
    group.extend_from_slice(&[0, 0, 1, 0]);
    group.extend_from_slice(&(SIZES.len() as u16).to_le_bytes());
    for (i, (size, img)) in SIZES.iter().zip(&images).enumerate() {
        // 256 is written as 0.
        let b = if *size >= 256 { 0 } else { *size as u8 };
        group.extend_from_slice(&[b, b, 0, 0]);
        group.extend_from_slice(&1u16.to_le_bytes());
        group.extend_from_slice(&32u16.to_le_bytes());
        group.extend_from_slice(&(img.len() as u32).to_le_bytes());
        group.extend_from_slice(&(i as u16 + 1).to_le_bytes());
    }
    resource(&mut res, 14, 1, 0x1030, &group);
    let dir = PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR"));
    // The same images as an .ico file, for looking at the icon.
    let mut ico = vec![0, 0, 1, 0];
    ico.extend_from_slice(&(SIZES.len() as u16).to_le_bytes());
    let mut offset = 6 + 16 * SIZES.len();
    for (size, img) in SIZES.iter().zip(&images) {
        let b = if *size >= 256 { 0 } else { *size as u8 };
        ico.extend_from_slice(&[b, b, 0, 0, 1, 0, 32, 0]);
        ico.extend_from_slice(&(img.len() as u32).to_le_bytes());
        ico.extend_from_slice(&(offset as u32).to_le_bytes());
        offset += img.len();
    }
    for img in &images {
        ico.extend_from_slice(img);
    }
    std::fs::write(dir.join("icon.ico"), ico).expect("writing icon.ico");
    let out = dir.join("icon.res");
    std::fs::write(&out, res).expect("writing icon.res");
    println!("cargo:rustc-link-arg-bins={}", out.display());
}
