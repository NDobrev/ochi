use anyhow::Result;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct Segment {
    pub name: String,
    pub base: u32,
    pub bytes: Vec<u8>,
    pub perms: &'static str, // e.g., "r-x"
    pub kind: &'static str,  // e.g., "raw"
}

#[derive(Debug, Clone)]
pub struct Image {
    pub segments: Vec<Segment>,
}

pub fn load_raw_bin(path: &Path, base: u32, skip: usize, len: Option<usize>) -> Result<Image> {
    let file = std::fs::read(path)?;
    anyhow::ensure!(skip <= file.len(), "--skip exceeds file size");
    let mut payload = &file[skip..];
    if let Some(lim) = len {
        anyhow::ensure!(lim <= payload.len(), "--len exceeds remaining file size after skip");
        payload = &payload[..lim];
    }
    let seg = Segment { name: "segment0".into(), base, bytes: payload.to_vec(), perms: "r-x", kind: "raw" };
    Ok(Image { segments: vec![seg] })
}

pub fn read_u8(img: &Image, addr: u32) -> Option<u8> {
    for s in &img.segments {
        let start = s.base;
        let end = s.base.wrapping_add(s.bytes.len() as u32);
        if addr >= start && addr < end {
            let off = (addr - start) as usize;
            return Some(s.bytes[off]);
        }
    }
    None
}

pub fn read_u16(img: &Image, addr: u32) -> Option<u16> {
    let b0 = read_u8(img, addr)?;
    let b1 = read_u8(img, addr.wrapping_add(1))?;
    Some(u16::from_le_bytes([b0, b1]))
}

pub fn read_u32(img: &Image, addr: u32) -> Option<u32> {
    let b0 = read_u8(img, addr)?;
    let b1 = read_u8(img, addr.wrapping_add(1))?;
    let b2 = read_u8(img, addr.wrapping_add(2))?;
    let b3 = read_u8(img, addr.wrapping_add(3))?;
    Some(u32::from_le_bytes([b0, b1, b2, b3]))
}

pub fn is_mapped(img: &Image, addr: u32) -> bool {
    img.segments.iter().any(|s| {
        let start = s.base;
        let end = s.base.wrapping_add(s.bytes.len() as u32);
        addr >= start && addr < end
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loader_maps_skip_and_len() {
        let cwd = std::env::current_dir().unwrap();
        let path = cwd.join("_test_bin.bin");
        std::fs::write(&path, [0u8, 1, 2, 3, 4, 5]).unwrap();
        let img = load_raw_bin(&path, 0x1000_0000, 2, Some(3)).unwrap();
        assert_eq!(img.segments.len(), 1);
        let s = &img.segments[0];
        assert_eq!(s.base, 0x1000_0000);
        assert_eq!(s.bytes, vec![2, 3, 4]);
        assert_eq!(read_u32(&img, 0x1000_0000).unwrap(), 0x00040302);
        assert!(read_u32(&img, 0x1000_0002 + 2).is_none());
        let _ = std::fs::remove_file(&path);
    }
}

