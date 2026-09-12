#!/usr/bin/env python3
"""Generate placeholder PNG trophies and the BluePhoenix app icon."""
from __future__ import annotations

import math
import struct
import zlib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
ACH = ROOT / "apps" / "desktop" / "src" / "assets" / "achievements"
ICONS = ROOT / "apps" / "desktop" / "src-tauri" / "icons"
PUBLIC = ROOT / "apps" / "desktop" / "public"


def png(width: int, height: int, rgba) -> bytes:
    def chunk(tag: bytes, data: bytes) -> bytes:
        return struct.pack(">I", len(data)) + tag + data + struct.pack(">I", zlib.crc32(tag + data) & 0xFFFFFFFF)

    raw = bytearray()
    for y in range(height):
        raw.append(0)
        for x in range(width):
            raw.extend(rgba(x, y))
    return b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", width, height, 8, 6, 0, 0, 0)) + chunk(b"IDAT", zlib.compress(bytes(raw), 9)) + chunk(b"IEND", b"")


def lerp(a, b, t):
    return tuple(int(a[i] + (b[i] - a[i]) * t) for i in range(3))


def draw_trophy(size: int, color, label: str, shape: str = "cup") -> bytes:
    cx, cy = size / 2, size / 2

    def rgba(x, y):
        dx, dy = x - cx, y - cy
        r = math.hypot(dx, dy) / (size / 2)
        bg = (8, 18, 22, 0 if r > 0.98 else 40)
        # cup body
        nx, ny = (x - cx) / size, (y - cy) / size
        inside = False
        if shape == "cup":
            inside = abs(nx) < 0.22 and -0.18 < ny < 0.12
            bowl = (nx * nx) / 0.08 + ((ny + 0.05) ** 2) / 0.05 < 1 and ny < 0.12
            stem = abs(nx) < 0.05 and 0.12 < ny < 0.28
            base = abs(nx) < 0.16 and 0.26 < ny < 0.34
            handles = (0.18 < abs(nx) < 0.30 and -0.12 < ny < 0.05)
            inside = bowl or stem or base or handles
        elif shape == "medal":
            inside = r < 0.42
        elif shape == "book":
            inside = abs(nx) < 0.28 and abs(ny) < 0.22
        elif shape == "rocket":
            inside = (abs(nx) < 0.10 and -0.28 < ny < 0.18) or (ny > 0.12 and abs(nx) < 0.18 - (ny - 0.12))
        elif shape == "check":
            inside = r < 0.40
        if inside:
            highlight = 1 - min(1, r)
            rgb = lerp(color, (255, 255, 255), 0.15 * highlight)
            return (*rgb, 255)
        # number / label in center-ish
        return bg

    data = png(size, size, rgba)
    # overlay label by redrawing a simple 5x7 font
    return overlay_label(size, color, label, shape)


def overlay_label(size: int, color, label: str, shape: str) -> bytes:
    font = {
        "0": ["01110", "10001", "10001", "10001", "10001", "10001", "01110"],
        "1": ["00100", "01100", "00100", "00100", "00100", "00100", "01110"],
        "2": ["01110", "10001", "00001", "00110", "01000", "10000", "11111"],
        "3": ["11110", "00001", "00001", "01110", "00001", "00001", "11110"],
        "4": ["00010", "00110", "01010", "10010", "11111", "00010", "00010"],
        "5": ["11111", "10000", "11110", "00001", "00001", "10001", "01110"],
        "6": ["01110", "10000", "11110", "10001", "10001", "10001", "01110"],
        "8": ["01110", "10001", "10001", "01110", "10001", "10001", "01110"],
        "A": ["01110", "10001", "10001", "11111", "10001", "10001", "10001"],
        "V": ["10001", "10001", "10001", "10001", "01010", "01010", "00100"],
    }
    cx, cy = size / 2, size / 2

    def glyph_pixels(text: str):
        scale = 3 if len(text) <= 2 else 2
        total_w = len(text) * 6 * scale
        start_x = int(cx - total_w / 2)
        start_y = int(cy - 3.5 * scale)
        pts = set()
        for i, ch in enumerate(text):
            g = font.get(ch, font["0"])
            for row, line in enumerate(g):
                for col, bit in enumerate(line):
                    if bit == "1":
                        for sy in range(scale):
                            for sx in range(scale):
                                pts.add((start_x + i * 6 * scale + col * scale + sx, start_y + row * scale + sy))
        return pts

    label_pts = glyph_pixels(label)

    def rgba(x, y):
        dx, dy = x - cx, y - cy
        r = math.hypot(dx, dy) / (size / 2)
        nx, ny = (x - cx) / size, (y - cy) / size
        inside = False
        if shape == "cup":
            bowl = (nx * nx) / 0.085 + ((ny + 0.02) ** 2) / 0.055 < 1 and ny < 0.14
            stem = abs(nx) < 0.045 and 0.12 < ny < 0.27
            base = abs(nx) < 0.17 and 0.26 < ny < 0.34
            handles = (0.20 < abs(nx) < 0.32 and -0.10 < ny < 0.06)
            inside = bowl or stem or base or handles
        elif shape == "medal":
            inside = r < 0.44
        elif shape == "book":
            inside = abs(nx) < 0.30 and abs(ny) < 0.24
        elif shape == "rocket":
            inside = (abs(nx) < 0.11 and -0.30 < ny < 0.16) or (ny > 0.10 and abs(nx) < 0.20 - (ny - 0.10))
        elif shape == "check":
            inside = r < 0.42
        if (x, y) in label_pts:
            return (255, 252, 240, 255)
        if inside:
            highlight = max(0.0, 1 - r)
            rgb = lerp(color, (230, 255, 250), 0.12 * highlight)
            edge = 1 if r > 0.9 else 0
            if edge:
                rgb = lerp(rgb, (255, 255, 255), 0.25)
            return (*rgb, 255)
        if r < 0.98:
            return (6, 16, 22, 18)
        return (0, 0, 0, 0)

    return png(size, size, rgba)


ACHIEVEMENTS = [
    ("trophy-first-project.png", (45, 180, 160), "1", "rocket"),
    ("trophy-1-hour.png", (80, 140, 170), "1", "cup"),
    ("trophy-10-hours.png", (40, 170, 150), "10", "cup"),
    ("trophy-25-hours.png", (30, 150, 190), "25", "cup"),
    ("trophy-50-hours.png", (180, 140, 40), "50", "cup"),
    ("trophy-100-hours.png", (210, 160, 50), "100", "cup"),
    ("trophy-10-todos.png", (70, 190, 130), "10", "check"),
    ("trophy-25-todos.png", (50, 170, 150), "25", "check"),
    ("trophy-100-todos.png", (40, 200, 170), "100", "check"),
    ("trophy-first-release.png", (90, 120, 210), "V", "rocket"),
    ("trophy-first-version.png", (120, 90, 210), "1", "rocket"),
    ("trophy-100-commits.png", (60, 110, 160), "100", "medal"),
    ("trophy-first-website.png", (40, 180, 200), "A", "medal"),
    ("trophy-first-github.png", (90, 100, 120), "1", "medal"),
    ("trophy-first-course.png", (70, 140, 200), "1", "book"),
    ("trophy-first-exam.png", (50, 160, 190), "1", "book"),
    ("trophy-perfect-exam.png", (220, 180, 40), "30", "book"),
    ("trophy-10-study-hours.png", (40, 150, 180), "10", "book"),
    ("trophy-100-study-hours.png", (30, 120, 170), "100", "book"),
    ("trophy-10-courses.png", (80, 130, 190), "10", "book"),
    ("trophy-30-cfu.png", (60, 160, 140), "30", "medal"),
    ("trophy-60-cfu.png", (50, 150, 170), "60", "medal"),
    ("trophy-100-cfu.png", (200, 160, 50), "100", "medal"),
    ("trophy-10-lessons.png", (70, 150, 160), "10", "book"),
]


def icon_rgba(x, y, size=1024):
    cx = cy = size / 2
    dx, dy = x - cx, y - cy
    r = math.hypot(dx, dy) / (size / 2)
    # dark navy to cyan disc
    t = min(1, r)
    rgb = lerp((8, 28, 48), (20, 90, 90), t)
    if 0.55 < r < 0.72:
        rgb = lerp((40, 210, 190), (20, 80, 90), (r - 0.55) / 0.17)
    if r > 0.92:
        a = int(255 * max(0, 1 - (r - 0.92) / 0.08))
        return (*rgb, a)
    return (*rgb, 255)


def main():
    ACH.mkdir(parents=True, exist_ok=True)
    ICONS.mkdir(parents=True, exist_ok=True)
    PUBLIC.mkdir(parents=True, exist_ok=True)
    pub_ach = PUBLIC / "assets" / "achievements"
    pub_ach.mkdir(parents=True, exist_ok=True)
    for name, color, label, shape in ACHIEVEMENTS:
        blob = overlay_label(128, color, label, shape)
        (ACH / name).write_bytes(blob)
        (pub_ach / name).write_bytes(blob)
        print("wrote", name)
    icon = png(1024, 1024, lambda x, y: icon_rgba(x, y, 1024))
    (ROOT / "apps" / "desktop" / "app-icon.png").write_bytes(icon)
    (PUBLIC / "icon.png").write_bytes(icon)
    print("wrote app-icon.png")


if __name__ == "__main__":
    main()
