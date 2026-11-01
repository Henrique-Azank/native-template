//! `cargo xtask ico <out.ico> <in.png>...`: pack PNG images into a Windows `.ico`.
//!
//! Windows Vista and later read PNG-compressed icon entries directly. The dimensions come from
//! each PNG's IHDR chunk; 256 px is stored as 0 in the directory entry per the ICO specification.

use std::path::Path;

/// Extracts width and height from a PNG's IHDR chunk.
pub fn png_size(png: &[u8]) -> Result<(u32, u32), String> {
    const SIG: &[u8] = b"\x89PNG\r\n\x1a\n";
    if png.len() < 24 || &png[..8] != SIG || &png[12..16] != b"IHDR" {
        return Err("not a valid PNG file".into());
    }
    let w = u32::from_be_bytes([png[16], png[17], png[18], png[19]]);
    let h = u32::from_be_bytes([png[20], png[21], png[22], png[23]]);
    Ok((w, h))
}

/// Packs square PNG slices (each <= 256 px) into a Windows `.ico` binary buffer.
pub fn pack(pngs: &[Vec<u8>]) -> Result<Vec<u8>, String> {
    if pngs.is_empty() || pngs.len() > u16::MAX as usize {
        return Err("need at least one PNG".into());
    }
    let mut out = Vec::new();
    out.extend_from_slice(&0u16.to_le_bytes()); // reserved
    out.extend_from_slice(&1u16.to_le_bytes()); // type: 1 = icon
    out.extend_from_slice(&(pngs.len() as u16).to_le_bytes());
    let mut offset = 6 + 16 * pngs.len();
    for png in pngs {
        let (w, h) = png_size(png)?;
        if w != h || w == 0 || w > 256 {
            return Err(format!(
                "icon images must be square and at most 256 px, got {w}x{h}"
            ));
        }
        let dim = if w == 256 { 0 } else { w as u8 };
        out.extend_from_slice(&[dim, dim, 0, 0]); // width, height, palette count, reserved
        out.extend_from_slice(&1u16.to_le_bytes()); // color planes
        out.extend_from_slice(&32u16.to_le_bytes()); // bits per pixel
        out.extend_from_slice(&(png.len() as u32).to_le_bytes());
        out.extend_from_slice(&(offset as u32).to_le_bytes());
        offset += png.len();
    }
    for png in pngs {
        out.extend_from_slice(png);
    }
    Ok(out)
}

pub fn run(args: &[&str]) -> Result<(), String> {
    let [out, inputs @ ..] = args else {
        return Err("usage: cargo xtask ico <out.ico> <in.png>...".into());
    };
    let pngs = inputs
        .iter()
        .map(|p| std::fs::read(p).map_err(|e| format!("read {p}: {e}")))
        .collect::<Result<Vec<_>, _>>()?;
    let ico = pack(&pngs)?;
    std::fs::write(Path::new(out), &ico).map_err(|e| format!("write {out}: {e}"))?;
    println!("{out}: {} images, {} bytes", pngs.len(), ico.len());
    Ok(())
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    fn fake_png(size: u32) -> Vec<u8> {
        let mut v = b"\x89PNG\r\n\x1a\n\0\0\0\x0dIHDR".to_vec();
        v.extend_from_slice(&size.to_be_bytes());
        v.extend_from_slice(&size.to_be_bytes());
        v.extend_from_slice(&[8, 6, 0, 0, 0, 1, 2, 3, 4]);
        v
    }

    #[test]
    fn packs_ico_entries() {
        let a = fake_png(16);
        let b = fake_png(256);
        let ico = pack(&[a.clone(), b.clone()]).unwrap();
        assert_eq!(&ico[..6], &[0, 0, 1, 0, 2, 0]);
        assert_eq!(&ico[6..8], &[16, 16]);
        assert_eq!(&ico[22..24], &[0, 0]);
        assert!(pack(&[]).is_err());
        assert!(pack(&[fake_png(512)]).is_err());
    }
}
