# Create and display charts with an AI agent

This guide is for users who want an AI agent to turn data or benchmark results
into an interactive native chart. Install the Feather Charts viewer once, then
give your agent the reusable prompt below. The agent writes a `.fchart` document
and opens it in the viewer; it does not need to compile an application for each
chart.

The prompt lives here so users and agents have a shared starting point. The
[format specification](fchart-format.md) remains the source of truth for file
fields, validation and limits. This page connects installation, file creation,
validation and display in a workflow that works with different AI agents.

## Install the viewer once on Linux

Install the released viewer from crates.io:

```sh
cargo install feather-charts-app --version 0.1.0 --locked --root "$HOME/.local"
"$HOME/.local/bin/feather-charts" --help
```

To install from a source checkout instead, run
`cargo install --path apps/feather-charts --locked --root "$HOME/.local"`
from the repository root.

This installs the `feather-charts` executable in `~/.local/bin`. If that directory
is on the agent's PATH, it can run `feather-charts` from any project. Otherwise,
give the agent the absolute executable path, such as
`/home/your-name/.local/bin/feather-charts`.

Opening the viewer needs a graphical desktop session. File-manager associations
are optional for this workflow; see the [Linux integration guide](../packaging/linux/README.md)
if you also want a desktop launcher or file-manager opening.

## Copy this prompt into your agent conversation

Replace `[DATA OR TASK]` with what you want plotted. Replace `[FORMAT_SPEC]`
with an absolute checkout path ending in `docs/fchart-format.md`, or the
[release's format specification](https://github.com/nimanoori22/feather-charts/blob/v0.2.0/docs/fchart-format.md).
Replace `[VIEWER]` with `feather-charts` or the absolute installed executable path.

```text
Create and display an interactive Feather Charts chart for [DATA OR TASK].

Read the format specification first:
[FORMAT_SPEC]

Write a self-contained version-1 .fchart document using the supplied data.
Preserve the actual numeric x/y values, series names and units. Choose a
supported line or histogram series; histograms require an explicit bar_width
in x-axis units. If the request exceeds the format's capabilities or limits,
explain the limitation rather than silently changing or discarding data.

Write to a new output file; do not overwrite an existing result unless asked.
For benchmarks, export after the measured section.

Validate the document against the format specification, then launch it using
the installed viewer, passing the absolute document path as one argument:
"[VIEWER]" "/absolute/path/to/results.fchart"

Report the saved file path and the command used to display it. If this session
cannot open a native window, provide the file and the launch command.
```

You can also copy the prompt into your project's agent instructions, such as
`AGENTS.md`, after replacing the placeholders. Installing the viewer alone does
not teach agents the format; they need this prompt or equivalent instructions.

## What the agent can create

Version 1 supports one pane, multiple line/histogram series sharing one y-unit,
and proportional linear numeric axes. The document embeds its data, so it can be
reopened later without rerunning the agent or benchmark. The read-only viewer
provides pan, zoom, Fit and a crosshair value readout.

Dates, categorical axes, OHLC documents and multiple panes are outside `.fchart`
v1. The embeddable Rust library supports additional chart types; those are
separate from what this document format accepts.

For working producer examples, validation commands and profiler CSV conversion,
see the [viewer and producer guide](fchart-viewer.md).
