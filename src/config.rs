use super::*;
use windows::{
    core::*,
    Win32::UI::Input::KeyboardAndMouse::*
};

pub const MOD_KEY: HOT_KEY_MODIFIERS = MOD_ALT;

pub const KEYS: [Key; 1] = [
    Key{mod_key:MOD_KEY,        key:10,     func:zoom,       arg:None}
];
