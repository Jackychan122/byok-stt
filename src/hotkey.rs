//! Hotkey configuration parsing — portable, no OS imports.
//!
//! A hotkey is a modifier ("ctrl" | "alt" | "shift" | "win"/"cmd" | "super")
//! plus a trigger key ("win"/"cmd"/"super" | "space" | a-z | 0-9 | f1-f12).
//! Defaults differ per OS (see config.rs): Windows Ctrl+Win, macOS Cmd+Space,
//! Linux Ctrl+Space. Windows-specific virtual-key mapping lives here as plain
//! constants so the module stays compilable on other platforms.

/// Windows virtual-key codes (also meaningful for other platforms' mappings).
pub const VK_SPACE: u16 = 0x20;
pub const VK_LWIN: u16 = 0x5B;
pub const VK_RWIN: u16 = 0x5C;

/// Left/right variants share one modifier id.
pub const MOD_CTRL: u8 = 0;
pub const MOD_ALT: u8 = 1;
pub const MOD_SHIFT: u8 = 2;
pub const MOD_WIN: u8 = 3;

/// Left/right virtual-key ranges for each modifier id.
const MOD_VKS: [(u16, u16); 4] = [
    (0xA2, 0xA3), // ctrl: VK_LCONTROL, VK_RCONTROL
    (0xA4, 0xA5), // alt: VK_LMENU, VK_RMENU
    (0xA0, 0xA1), // shift: VK_LSHIFT, VK_RSHIFT
    (VK_LWIN, VK_RWIN),
];

pub fn parse_modifier(s: &str) -> Option<u8> {
    match s.trim().to_lowercase().as_str() {
        "ctrl" | "control" => Some(MOD_CTRL),
        "alt" | "option" | "menu" => Some(MOD_ALT),
        "shift" => Some(MOD_SHIFT),
        "win" | "cmd" | "super" | "meta" => Some(MOD_WIN),
        _ => None,
    }
}

/// Map a trigger-key name to a virtual-key code.
pub fn parse_key(s: &str) -> Option<u16> {
    let s = s.trim().to_lowercase();
    match s.as_str() {
        "win" | "cmd" | "super" | "meta" => return Some(VK_LWIN),
        "space" | "spacebar" => return Some(VK_SPACE),
        "tab" => return Some(0x09),
        "insert" => return Some(0x2D),
        "home" => return Some(0x24),
        "pageup" => return Some(0x21),
        "pagedown" => return Some(0x22),
        _ => {}
    }
    if s.len() == 1 {
        let c = s.as_bytes()[0];
        if c.is_ascii_lowercase() {
            return Some(0x41 + (c - b'a') as u16); // VK_A..VK_Z
        }
        if c.is_ascii_digit() {
            return Some(0x30 + (c - b'0') as u16); // VK_0..VK_9
        }
    }
    if let Some(num) = s.strip_prefix('f') {
        if let Ok(n) = num.parse::<u8>() {
            if (1..=12).contains(&n) {
                return Some(0x70 + (n - 1) as u16); // VK_F1..VK_F12
            }
        }
    }
    None
}

/// Does a low-level hook virtual-key belong to the configured modifier?
pub fn is_modifier_vk(mod_id: u8, vk: u16) -> bool {
    MOD_VKS
        .get(mod_id as usize)
        .map(|&(lo, hi)| vk == lo || vk == hi)
        .unwrap_or(false)
}

/// Does a low-level hook virtual-key belong to the configured trigger key?
/// (Win as trigger matches both left and right Win.)
pub fn is_key_vk(key_vk: u16, vk: u16) -> bool {
    if key_vk == VK_LWIN {
        vk == VK_LWIN || vk == VK_RWIN
    } else {
        vk == key_vk
    }
}

/// True when Win participates in the hotkey (either side) — used to decide
/// whether the OS should be shielded from Win releases mid-session.
pub fn uses_win(mod_id: u8, key_vk: u16) -> bool {
    mod_id == MOD_WIN || key_vk == VK_LWIN
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_all_modifiers() {
        assert_eq!(parse_modifier("ctrl"), Some(MOD_CTRL));
        assert_eq!(parse_modifier("Ctrl"), Some(MOD_CTRL));
        assert_eq!(parse_modifier("alt"), Some(MOD_ALT));
        assert_eq!(parse_modifier("shift"), Some(MOD_SHIFT));
        assert_eq!(parse_modifier("win"), Some(MOD_WIN));
        assert_eq!(parse_modifier("cmd"), Some(MOD_WIN)); // macOS alias
        assert_eq!(parse_modifier("super"), Some(MOD_WIN)); // Linux alias
        assert_eq!(parse_modifier("bogus"), None);
    }

    #[test]
    fn parses_trigger_keys() {
        assert_eq!(parse_key("space"), Some(VK_SPACE));
        assert_eq!(parse_key("Spacebar"), Some(VK_SPACE));
        assert_eq!(parse_key("win"), Some(VK_LWIN));
        assert_eq!(parse_key("a"), Some(0x41));
        assert_eq!(parse_key("Z"), Some(0x5A));
        assert_eq!(parse_key("0"), Some(0x30));
        assert_eq!(parse_key("f12"), Some(0x7B));
        assert_eq!(parse_key("F1"), Some(0x70));
        assert_eq!(parse_key("bogus"), None);
    }

    #[test]
    fn modifier_matches_left_and_right_variants() {
        assert!(is_modifier_vk(MOD_CTRL, 0xA2)); // LCtrl
        assert!(is_modifier_vk(MOD_CTRL, 0xA3)); // RCtrl
        assert!(is_modifier_vk(MOD_WIN, 0x5B)); // LWin
        assert!(!is_modifier_vk(MOD_CTRL, 0x20)); // Space is not Ctrl
    }

    #[test]
    fn trigger_matches_both_win_keys() {
        assert!(is_key_vk(VK_LWIN, 0x5B));
        assert!(is_key_vk(VK_LWIN, 0x5C)); // RWin matches a "win" trigger
        assert!(is_key_vk(VK_SPACE, VK_SPACE));
        assert!(!is_key_vk(VK_SPACE, 0x5B));
    }

    #[test]
    fn win_usage_detection() {
        assert!(uses_win(MOD_WIN, VK_SPACE)); // Win as modifier
        assert!(uses_win(MOD_CTRL, VK_LWIN)); // Win as trigger (Ctrl+Win)
        assert!(!uses_win(MOD_CTRL, VK_SPACE)); // Ctrl+Space keeps Win free
    }
}
