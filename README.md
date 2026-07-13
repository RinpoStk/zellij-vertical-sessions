# zellij-vertical-sessions

[中文](README.zh.md)

A lightweight Zellij plugin that displays sessions in a vertical sidebar, inspired by [Vertical Tab Bar for Zellij](https://github.com/cfal/zellij-vertical-tabs).

## Notice

This project was generated entirely by AI. Please audit it yourself 😁.

## Permission File (Required)

Before starting the plugin, add its permissions to Zellij's `permissions.kdl` file.

- macOS: `~/Library/Caches/org.Zellij-Contributors.Zellij/permissions.kdl`
- Linux: `~/.cache/zellij/permissions.kdl`

Create the directory or file if it does not exist. Add the following entry and replace the path with the actual absolute path to the WASM file.

```kdl
"<plugin-path>" {
    ChangeApplicationState
    ReadApplicationState
}
```

## Installation

Copy the WASM plugin to `~/.config/zellij/plugins`, then copy the layout files from `examples/` to `~/.config/zellij/layouts/`.

## Usage

Start Zellij with an example layout:

```sh
zellij --layout vertical-sessions-left
# or
zellij --layout vertical-sessions-right
```

In Session mode, use `Up` / `Down` to select a session and press `Enter` to switch. Mouse clicks and scrolling are also supported.

See [`examples/`](examples/) for layout and style configuration.
