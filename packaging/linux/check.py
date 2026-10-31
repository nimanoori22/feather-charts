#!/usr/bin/env python3
"""Validate MIME registration in a disposable prefix; never modify user defaults."""
import os
from pathlib import Path
import shutil
import subprocess
import tempfile


def main():
    repository = Path(__file__).resolve().parents[2]
    assets = repository / "packaging/linux"
    subprocess.run(["desktop-file-validate", str(assets / "feather-charts.desktop")], check=True)
    with tempfile.TemporaryDirectory(prefix="fchart-xdg-") as temporary:
        root = Path(temporary)
        data, config = root / "data", root / "config"
        (data / "applications").mkdir(parents=True)
        (data / "mime/packages").mkdir(parents=True)
        config.mkdir()
        shutil.copy(assets / "feather-charts.desktop", data / "applications")
        shutil.copy(assets / "feather-chart.xml", data / "mime/packages")
        env = dict(os.environ, XDG_DATA_HOME=str(data), XDG_CONFIG_HOME=str(config))
        subprocess.run(["update-mime-database", str(data / "mime")], env=env, check=True, capture_output=True)
        subprocess.run(["update-desktop-database", str(data / "applications")], env=env, check=True)
        fixture = root / "results with spaces.fchart"
        shutil.copy(repository / "examples/fchart/minimal.fchart", fixture)
        result = subprocess.run(["gio", "info", "-a", "standard::content-type", str(fixture)],
                                env=env, check=True, capture_output=True, text=True)
        assert "standard::content-type: application/x-feather-chart" in result.stdout, result.stdout
        assert not (config / "mimeapps.list").exists()
    assert not root.exists()
    print("Desktop entry, MIME cache, path with spaces and disposable-prefix cleanup passed.")
    print("Native launcher and file-manager interaction require a graphical session check.")


if __name__ == "__main__":
    main()
