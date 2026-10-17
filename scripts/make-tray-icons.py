#!/usr/bin/env python3
"""Tray icon variants for low / critical disk space.

Reads the broom glyph from app/src-tauri/icons/tray-template.png (a 44x44
black-on-transparent template; macOS tints it itself) and writes the same
glyph filled with a warning colour:

  tray-low.png       amber   #E0A200  (free space < 20 %)
  tray-critical.png  red     #D9342B  (free space < 5 %)

Pure Python (zlib + struct), no Pillow needed. Run from the repo root:

  python3 scripts/make-tray-icons.py
"""

import struct
import sys
import zlib
from pathlib import Path

ICONS = Path(__file__).resolve().parent.parent / "app" / "src-tauri" / "icons"
VARIANTS = {
    "tray-low.png": (0xE0, 0xA2, 0x00),
    "tray-critical.png": (0xD9, 0x34, 0x2B),
}


def read_png_rgba(path: Path):
    data = path.read_bytes()
    if data[:8] != b"\x89PNG\r\n\x1a\n":
        raise SystemExit(f"{path} is not a PNG")
    pos, idat, width, height = 8, b"", 0, 0
    while pos < len(data):
        (length,) = struct.unpack(">I", data[pos : pos + 4])
        kind = data[pos + 4 : pos + 8]
        chunk = data[pos + 8 : pos + 8 + length]
        if kind == b"IHDR":
            width, height, depth, ctype = struct.unpack(">IIBB", chunk[:10])
            if depth != 8 or ctype != 6:
                raise SystemExit("expected 8-bit RGBA")
        elif kind == b"IDAT":
            idat += chunk
        pos += 12 + length
    raw = zlib.decompress(idat)
    stride, bpp = width * 4, 4
    rows, prev, i = [], bytearray(stride), 0
    for _ in range(height):
        f, line = raw[i], bytearray(raw[i + 1 : i + 1 + stride])
        i += 1 + stride
        for x in range(stride):
            a = line[x - bpp] if x >= bpp else 0
            b = prev[x]
            c = prev[x - bpp] if x >= bpp else 0
            if f == 1:
                line[x] = (line[x] + a) & 255
            elif f == 2:
                line[x] = (line[x] + b) & 255
            elif f == 3:
                line[x] = (line[x] + (a + b) // 2) & 255
            elif f == 4:
                p = a + b - c
                pa, pb, pc = abs(p - a), abs(p - b), abs(p - c)
                line[x] = (line[x] + (a if pa <= pb and pa <= pc else b if pb <= pc else c)) & 255
        rows.append(bytes(line))
        prev = line
    return width, height, rows


def write_png_rgba(path: Path, width: int, height: int, rows) -> None:
    def chunk(kind: bytes, body: bytes) -> bytes:
        return struct.pack(">I", len(body)) + kind + body + struct.pack(">I", zlib.crc32(kind + body) & 0xFFFFFFFF)

    raw = b"".join(b"\x00" + row for row in rows)
    png = b"\x89PNG\r\n\x1a\n"
    png += chunk(b"IHDR", struct.pack(">IIBBBBB", width, height, 8, 6, 0, 0, 0))
    png += chunk(b"IDAT", zlib.compress(raw, 9))
    png += chunk(b"IEND", b"")
    path.write_bytes(png)


def main() -> int:
    width, height, rows = read_png_rgba(ICONS / "tray-template.png")
    for name, (r, g, b) in VARIANTS.items():
        out = []
        for row in rows:
            px = bytearray(row)
            for x in range(0, len(px), 4):
                if px[x + 3]:
                    px[x], px[x + 1], px[x + 2] = r, g, b
            out.append(bytes(px))
        write_png_rgba(ICONS / name, width, height, out)
        print(f"wrote {ICONS / name}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
