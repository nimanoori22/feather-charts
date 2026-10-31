# Linux integration (opt-in)

Install the executable with `cargo install feather-charts-app --version 0.1.0 --locked`.
To build/install from this checkout instead, use `cargo install --path apps/feather-charts --locked`.
This installs package `feather-charts-app` and executable `feather-charts`.
A running graphical Linux session, X11/Wayland libraries, fonts and either the
software renderer or a WGPU-compatible Vulkan/OpenGL driver are required.
The launcher uses `Exec=feather-charts %f`: one local file argument, including
paths with spaces. Ensure the executable's directory is in the desktop session's
PATH. No command shell interprets document paths.

Prepare user-owned desktop assets (from repository root):

```sh
install -Dm644 packaging/linux/feather-charts.desktop "$HOME/.local/share/applications/feather-charts.desktop"
install -Dm644 packaging/linux/feather-chart.xml "$HOME/.local/share/mime/packages/feather-chart.xml"
update-mime-database "$HOME/.local/share/mime"
update-desktop-database "$HOME/.local/share/applications"
```

These register availability without changing a default association. If explicitly
wanted, record `xdg-mime query default application/x-feather-chart` first, then
use `xdg-mime default feather-charts.desktop application/x-feather-chart`.
Restore the previously recorded desktop ID to undo a default change.

Uninstall only these user-owned files, refresh the same caches, then
`cargo uninstall feather-charts-app`. Do not remove another application's files.

## Isolated integration test

Run `python packaging/linux/check.py` for a repeatable isolated MIME/cache check.
GIO uses the shared MIME database; generic `xdg-mime` on minimal desktops may fall
back to libmagic and report JSON regardless of installed glob registrations.

Use a temporary directory as `XDG_DATA_HOME` and `XDG_CONFIG_HOME`, with a
user-owned temporary `bin` on PATH. Copy the built binary there and install the
two assets under that data prefix, then run the cache refresh commands there.
Use `desktop-file-validate` and `update-mime-database` where available. Check
`xdg-mime query filetype 'path with spaces.fchart'`; launch with
`gio launch "$XDG_DATA_HOME/applications/feather-charts.desktop" 'path with spaces.fchart'`.
Test file-manager Open With in the same isolated environment. Merely launching
from a terminal is not file-manager acceptance. Delete the temporary prefix to
uninstall the test. Never point these tests at the user's real defaults.
