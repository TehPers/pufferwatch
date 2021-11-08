use std::{fmt::Debug, hash::Hash};

pub trait IconPack: Clone + Copy + PartialEq + Eq + Debug + Hash + Default + 'static {
    const CONTROL_ICON: &'static str;
    const ALT_ICON: &'static str;
    const SHIFT_ICON: &'static str;

    const LEFT_ICON: &'static str;
    const RIGHT_ICON: &'static str;
    const UP_ICON: &'static str;
    const DOWN_ICON: &'static str;
    const INSERT_ICON: &'static str;
    const NULL_ICON: &'static str;
    const BACKSPACE_ICON: &'static str;
    const ENTER_ICON: &'static str;
    const HOME_ICON: &'static str;
    const END_ICON: &'static str;
    const PAGEUP_ICON: &'static str;
    const PAGEDOWN_ICON: &'static str;
    const TAB_ICON: &'static str;
    const BACKTAB_ICON: &'static str;
    const DELETE_ICON: &'static str;
    const ESC_ICON: &'static str;
    const SPACE_ICON: &'static str;

    const UP_DOWN: &'static str;
    const LEFT_RIGHT: &'static str;
    const ARROWS: &'static str;
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash, Default)]
pub struct UnicodeIconPack;

impl IconPack for UnicodeIconPack {
    const CONTROL_ICON: &'static str = if cfg!(target_os = "macos") {
        "\u{2318}"
    } else {
        "\u{2303}"
    };
    const ALT_ICON: &'static str = "\u{2325}";
    const SHIFT_ICON: &'static str = "\u{21e7}";

    const LEFT_ICON: &'static str = "\u{2190}";
    const RIGHT_ICON: &'static str = "\u{2192}";
    const UP_ICON: &'static str = "\u{2191}";
    const DOWN_ICON: &'static str = "\u{2193}";
    const INSERT_ICON: &'static str = "INS";
    const NULL_ICON: &'static str = "NUL";
    const BACKSPACE_ICON: &'static str = "\u{232b}";
    const ENTER_ICON: &'static str = "\u{23ce}";
    const HOME_ICON: &'static str = "\u{2196}";
    const END_ICON: &'static str = "\u{2198}";
    const PAGEUP_ICON: &'static str = "\u{21de}";
    const PAGEDOWN_ICON: &'static str = "\u{21df}";
    const TAB_ICON: &'static str = "\u{21e5}";
    const BACKTAB_ICON: &'static str = "\u{21e4}";
    const DELETE_ICON: &'static str = "\u{2326}";
    const ESC_ICON: &'static str = "\u{238b}";
    const SPACE_ICON: &'static str = "\u{2423}";

    const UP_DOWN: &'static str = "\u{2191}\u{2193}";
    const LEFT_RIGHT: &'static str = "\u{2192}\u{2190}";
    const ARROWS: &'static str = "\u{2191}\u{2193}\u{2192}\u{2190}";
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash, Default)]
pub struct NonUnicodeIconPack;

impl IconPack for NonUnicodeIconPack {
    const CONTROL_ICON: &'static str = "CTRL+";
    const ALT_ICON: &'static str = "ALT+";
    const SHIFT_ICON: &'static str = "SHFT+";

    const LEFT_ICON: &'static str = UnicodeIconPack::LEFT_ICON;
    const RIGHT_ICON: &'static str = UnicodeIconPack::RIGHT_ICON;
    const UP_ICON: &'static str = UnicodeIconPack::UP_ICON;
    const DOWN_ICON: &'static str = UnicodeIconPack::DOWN_ICON;
    const INSERT_ICON: &'static str = "INS";
    const NULL_ICON: &'static str = "NUL";
    const BACKSPACE_ICON: &'static str = "BKSP";
    const ENTER_ICON: &'static str = "ENTR";
    const HOME_ICON: &'static str = "HOME";
    const END_ICON: &'static str = "END";
    const PAGEUP_ICON: &'static str = "PGUP";
    const PAGEDOWN_ICON: &'static str = "PGDN";
    const TAB_ICON: &'static str = "TAB";
    const BACKTAB_ICON: &'static str = "BTAB";
    const DELETE_ICON: &'static str = "DEL";
    const ESC_ICON: &'static str = "ESC";
    const SPACE_ICON: &'static str = "SPC";

    const UP_DOWN: &'static str = UnicodeIconPack::UP_DOWN;
    const LEFT_RIGHT: &'static str = UnicodeIconPack::LEFT_RIGHT;
    const ARROWS: &'static str = UnicodeIconPack::ARROWS;
}

#[cfg(not(target_os = "windows"))]
pub type DefaultIconPack = UnicodeIconPack;

#[cfg(target_os = "windows")]
pub type DefaultIconPack = NonUnicodeIconPack;
