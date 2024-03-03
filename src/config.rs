use super::*;
use windows::{
    core::*,
    Win32::UI::Input::KeyboardAndMouse::*
};

pub const MOD_KEY: HOT_KEY_MODIFIERS = MOD_ALT;

pub const KEYS: [Key; 6] = [
    Key{mod_key:MOD_KEY,        key:'Q',     func:DwmrApp::quit,                    arg:None},
    Key{mod_key:MOD_KEY,        key:'Z',     func:DwmrApp::zoom,                    arg:None},
    Key{mod_key:MOD_KEY,        key:'J',     func:DwmrApp::focus_stack,             arg:Some(Arg{i:  1})},
    Key{mod_key:MOD_KEY,        key:'K',     func:DwmrApp::focus_stack,             arg:Some(Arg{i: -1})},
    Key{mod_key:MOD_KEY,        key:'H',     func:DwmrApp::focus_monitor,           arg:Some(Arg{i:  1})},
    Key{mod_key:MOD_KEY,        key:'L',     func:DwmrApp::focus_monitor,           arg:Some(Arg{i: -1})},
];
