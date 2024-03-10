use super::*;

macro_rules! tag_keys {
    ($key:expr, $tag:expr) => {
        [
            Key{mod_key:MOD_KEY,                        key:$key,       func:DwmrApp::view,             arg:Some(Arg{ui: 1 << $tag})},
            Key{mod_key:MOD_KEY|MOD_CONTROL,            key:$key,       func:DwmrApp::toggle_view,      arg:Some(Arg{ui: 1 << $tag})},
            Key{mod_key:MOD_KEY|MOD_SHIFT,              key:$key,       func:DwmrApp::tag,              arg:Some(Arg{ui: 1 << $tag})},
            Key{mod_key:MOD_KEY|MOD_CONTROL|MOD_SHIFT,  key:$key,       func:DwmrApp::toggle_tag,       arg:Some(Arg{ui: 1 << $tag})},
        ]
    };
}

pub const TAGS: [PCWSTR; 9] = [
    w!("1"),
    w!("2"),
    w!("3"),
    w!("4"),
    w!("5"),
    w!("6"),
    w!("7"),
    w!("8"),
    w!("9"),
];

pub const EXCLUDE_DEBUGGED_WINDOW: bool = true;

pub const MOD_KEY: HOT_KEY_MODIFIERS = MOD_ALT;

pub const KEYS: [Key; 6] = [
    Key{mod_key:MOD_KEY,        key:'Q',     func:DwmrApp::quit,                    arg:None},
    Key{mod_key:MOD_KEY,        key:'Z',     func:DwmrApp::zoom,                    arg:None},
    Key{mod_key:MOD_KEY,        key:'J',     func:DwmrApp::focus_stack,             arg:Some(Arg{i:  1})},
    Key{mod_key:MOD_KEY,        key:'K',     func:DwmrApp::focus_stack,             arg:Some(Arg{i: -1})},
    Key{mod_key:MOD_KEY,        key:'H',     func:DwmrApp::focus_monitor,           arg:Some(Arg{i:  1})},
    Key{mod_key:MOD_KEY,        key:'L',     func:DwmrApp::focus_monitor,           arg:Some(Arg{i: -1})},
];

lazy_static! {
     pub static ref TAG_KEYS: [[Key; 4]; 9] = [
        tag_keys!('1', 0),
        tag_keys!('2', 1),
        tag_keys!('3', 2),
        tag_keys!('4', 3),
        tag_keys!('5', 4),
        tag_keys!('6', 5),
        tag_keys!('7', 6),
        tag_keys!('8', 7),
        tag_keys!('9', 8),
    ];
}
