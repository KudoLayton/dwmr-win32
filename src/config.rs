use windows::core::*;
use windows::Win32::Graphics::Direct2D::Common::D2D1_COLOR_F;
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

pub const BAR_PADDING: f32 = 10.0;
pub const BAR_FONT: PCWSTR = w!("Arial");
pub const BAR_UNSELECTED_WINDOW_MARK: PCWSTR = w!("□");
pub const BAR_SELECTED_WINDOW_MARK: PCWSTR = w!("■");

pub const EXCLUDE_DEBUGGED_WINDOW: bool = true;

pub const MOD_KEY: HOT_KEY_MODIFIERS = MOD_ALT;

pub const BAR_TRANSPARENCY: f32 = 0.8;

pub const BAR_COLOR_BACKGROUND      :D2D1_COLOR_F   = D2D1_COLOR_F{ r:  40.0 / 255.0, g:  44.0 / 255.0, b:  55.0 / 255.0, a: 1.0 };
pub const BAR_COLOR_SELECTED_BOX    :D2D1_COLOR_F   = D2D1_COLOR_F{ r:  43.0 / 255.0, g: 144.0 / 255.0, b: 217.0 / 255.0, a: 1.0 };
pub const BAR_COLOR_UNSELECTED_TEXT :D2D1_COLOR_F   = D2D1_COLOR_F{ r: 155.0 / 255.0, g: 174.0 / 255.0, b: 200.0 / 255.0, a: 1.0 };
pub const BAR_COLOR_SELECTED_TEXT   :D2D1_COLOR_F   = D2D1_COLOR_F{ r: 217.0 / 255.0, g: 225.0 / 255.0, b: 232.0 / 255.0, a: 1.0 };

lazy_static! {
    pub static ref RULES: [Rule; 4] = [
        Rule{title: None,       class: None,        process_filename: Some("KakaoTalk".to_string()),         is_floating: true,      tags: 1 << 0},
        Rule{title: Some("화면 속 화면".to_string()),       class: None,        process_filename: None,         is_floating: true,      tags: 1 << 0},
        Rule{title: None,       class: None,        process_filename: Some("steamapps".to_string()),         is_floating: true,      tags: 1 << 0},
        Rule{title: None,       class: None,        process_filename: Some("mstsc".to_string()),         is_floating: true,      tags: 1 << 0},
    ];

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

    pub static ref KEYS: [Key; 13] = [
        Key{mod_key:MOD_KEY,                    key:'Q',     func:DwmrApp::quit,                    arg:None},
        Key{mod_key:MOD_KEY,                    key:'Z',     func:DwmrApp::zoom,                    arg:None},
        Key{mod_key:MOD_KEY,                    key:'F',     func:DwmrApp::toggle_float,            arg:None},
        Key{mod_key:MOD_KEY,                    key:'J',     func:DwmrApp::focus_stack,             arg:Some(Arg{i:  1})},
        Key{mod_key:MOD_KEY,                    key:'K',     func:DwmrApp::focus_stack,             arg:Some(Arg{i: -1})},
        Key{mod_key:MOD_KEY,                    key:'T',     func:DwmrApp::set_layout,              arg:Some(Arg{l:  Layout::Tile(Default::default())})},
        Key{mod_key:MOD_KEY,                    key:'S',     func:DwmrApp::set_layout,              arg:Some(Arg{l:  Layout::Stack(Default::default())})},
        Key{mod_key:MOD_KEY,                    key:'H',     func:DwmrApp::focus_monitor,           arg:Some(Arg{i:  1})},
        Key{mod_key:MOD_KEY,                    key:'L',     func:DwmrApp::focus_monitor,           arg:Some(Arg{i: -1})},
        Key{mod_key:MOD_KEY,                    key:'I',     func:DwmrApp::set_monitor_factor,      arg:Some(Arg{f:  0.05})},
        Key{mod_key:MOD_KEY,                    key:'D',     func:DwmrApp::set_monitor_factor,      arg:Some(Arg{f: -0.05})},
        Key{mod_key:MOD_KEY|MOD_SHIFT,          key:'H',     func:DwmrApp::tag_monitor,             arg:Some(Arg{i:  1})},
        Key{mod_key:MOD_KEY|MOD_SHIFT,          key:'L',     func:DwmrApp::tag_monitor,             arg:Some(Arg{i: -1})},
    ];
}
