//! Read-only drive names for the desktop folder picker. No volume readiness I/O.
pub fn roots_from_mask(mask: u32) -> Vec<String> {
    (0u8..26).filter(|index| mask & (1 << index) != 0)
        .map(|index| format!("{}:\\", char::from(b'A' + index))).collect()
}
#[cfg(windows)]
pub fn drive_roots() -> std::io::Result<Vec<String>> {
    let mask = unsafe { windows_sys::Win32::Storage::FileSystem::GetLogicalDrives() };
    if mask == 0 { return Err(std::io::Error::last_os_error()); }
    Ok(roots_from_mask(mask))
}
#[cfg(not(windows))]
pub fn drive_roots() -> std::io::Result<Vec<String>> { Ok(vec!["/".into()]) }
#[cfg(test)]
mod tests {
    #[test]
    fn mask_names_need_no_filesystem_probe() {
        assert_eq!(super::roots_from_mask((1 << 2) | (1 << 25)), ["C:\\", "Z:\\"]);
        assert!(super::roots_from_mask(0).is_empty());
    }
}
