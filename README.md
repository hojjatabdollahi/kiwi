<p align="center">
  <img src="data/icons/kiwi-on.svg" alt="Kiwi Logo" width="128">
</p>

# Kiwi

A key visualizer for [COSMIC DE](https://github.com/pop-os/cosmic-epoch). Shows an overlay of your keystrokes, mouse clicks, and gestures.

![Kiwi Demo](data/kiwi.gif)

## Features

- Real-time keystroke visualization overlay
- Mouse button and scroll wheel display
- Touchpad gesture recognition (swipes, holds)
- System tray integration
- Configurable position, size, colors, and ...
- Multiple color palettes

## Requirements

- COSMIC DE
- Rust
- [just](https://github.com/casey/just)
- User must be in the `input` group (for libinput access)

## Build

```bash
just build-release
```

## Install

```bash
sudo just install
```

This installs to `/usr/bin/kiwi` along with desktop entry and icons.

## Uninstall

```bash
sudo just uninstall
```

## Setup

### Add yourself to the input group

Kiwi uses libinput to capture keystrokes, which requires read access to `/dev/input/*` devices:

```bash
sudo usermod -aG input $USER
```

**Log out and log back in** for the group change to take effect.

### Verify group membership

```bash
groups | grep input
```

## Usage

Launch Kiwi from your application menu or run:

```bash
kiwi
```

Kiwi runs as a tray icon. Click the tray icon to toggle the overlay. Right-click for settings and quit options.

## Configuration

Settings are stored via cosmic-config and can be accessed through the tray icon menu.

## Security

Adding yourself to the `input` group grants read access to all input devices (`/dev/input/*`). This means any program you run can read all keystrokes, including passwords. Only do this on systems you trust and where you control what software runs.

## Why Kiwi?
It's a Key Visualizer!

## License

GPL-3.0-or-later
