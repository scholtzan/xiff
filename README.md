# Xiff - Plugin to show file diffs in Xi Editor

> Note: This project is work in progress and currently not stable nor supported by default by xi-mac.

This plugin shows file diffs in the gutter of Xi Editor.

![Screenshot](screenshot.png)

## Installation

tldr; `make install`.

To install this plugin, the plugin manifest must be placed in a new directory under
$XI_CONFIG_DIR/plugins, where $XI_CONFIG_DIR is the path passed by your client
to xi core via the `client_started` RPC's `config_dir` field, on startup.
On MacOS, by default, $XI_CONFIG_DIR is located at ~/Library/Application Support/XiEditor.

Additionally, the compiled binary should be placed in a `bin/` subdir of the
directory containing the manifest. (This is the default; the location can be
changed in the manifest.)

Currently this plugin requires a special branch of xi-mac which adds support for gutter annotations: https://github.com/scholtzan/xi-mac/tree/diff-annotations
