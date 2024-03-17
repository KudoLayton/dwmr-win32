# dwmr-win32: dwm port for Windows

dwmr-win32 brings the renowned simplicity, extendability, and efficiency of the [suckless dwm (Dynamic Window Manager)](https://dwm.suckless.org/) to Windows by rewriting it in Rust. This project aims to provide Windows users with the streamlined desktop experience that dwm is known for, blending dwm's minimalist design principles with the unique aspects of the Windows environment.

## Key Features

- **Tiling layout**
- **Vertical stack layout**
- **Tagging system**
- **Status bar**
- **Mouse controls**
- **Floating mode**

## Getting Started

Rust's Cargo build system is required for compiling dwmr-win32, ensuring a smooth build process.

### Prerequisites

- Rust and Cargo installed on your system

### Installation and Building

1. Clone this repository to your local machine:

```Bash
git clone https://github.com/KudoLayton/dwmr-win32
```

2. Navigate to the project directory:

```Bash
cd dwmr-win32
```

3. Compile the project using Cargo:

```Bash
cargo build --release
```

## Default Keybindings

dwmr-win32 offers a variety of keybindings to efficiently manage windows, workspaces, and the application itself. The default modifier key is set to `ALT`. Here's a summary of the essential keybindings:

### Window Management
- **ALT + J/K**: Focus the next/previous window.
- **ALT + F**: Toggle floating mode for the active window.
- **ALT + T**: Set the layout to tiling.
- **ALT + S**: Set the layout to vertical stacking.

### Tag Management
- **ALT + [1-9]**: View tag (workspace) [1-9].
- **ALT + SHIFT + [1-9]**: Assign the active window to tag [1-9].
- **ALT + CTRL + [1-9]**: Toggle the view of tag [1-9].
- **ALT + CTRL + SHIFT + [1-9]**: Toggle the assigned tag of the active window.

### Monitor Management
- **ALT + H/L**: Focus the next/previous monitor.
- **ALT + SHIFT + H/L**: Move the active window to the next/previous monitor.
- **ALT + I/D**: Increase/decrease the size of the master area.

### Application
- **ALT + Q**: Quit dwmr-win32.
- **ALT + Z**: Zoom (toggle the master area between the active window and the previous one).

These keybindings are defined in the `src/config.rs` file and can be customized to suit your preferences.

### Configuration

To customize your dwm experience, modify the `src/config.rs` file. This allows for personal adjustments like key bindings, window rules, and aesthetic preferences.

**Note**: It's necessary to recompile the project after making any changes to `config.rs`. Simply run `cargo build --release` again to apply your changes.

## Usage

Launch the compiled executable to start enjoying a minimalist and efficient window management experience with dwmr-win32 on your Windows environment.


## License

This project is distributed under the MIT License. For more details, see the [LICENSE](LICENSE) file.

## Acknowledgements

Our heartfelt thanks go to the suckless community and the original creators of dwm. This port to Windows, dwmr-win32, builds on their pioneering work, aiming to extend their minimalist design philosophy to Windows users worldwide.

