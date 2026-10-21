"""生成 SkillBox 应用图标（纯标准库，无需 Pillow）。

输出 1024x1024 RGBA PNG：圆角渐变方块 + 三条白色圆角横条（象征一组 Skills）。
"""
import os
import struct
import zlib


def rounded_rect_alpha(x, y, w, h, r):
    """返回判断点是否在圆角矩形内的函数"""
    x2, y2 = x + w, y + h
    return lambda px, py: (
        (x + r <= px <= x2 - r and y <= py <= y2)
        or (x <= px <= x2 and y + r <= py <= y2 - r)
        or ((px - (x + r)) ** 2 + (py - (y + r)) ** 2 <= r * r and px < x + r and py < y + r)
        or ((px - (x2 - r)) ** 2 + (py - (y + r)) ** 2 <= r * r and px > x2 - r and py < y + r)
        or ((px - (x + r)) ** 2 + (py - (y2 - r)) ** 2 <= r * r and px < x + r and py > y2 - r)
        or ((px - (x2 - r)) ** 2 + (py - (y2 - r)) ** 2 <= r * r and px > x2 - r and py > y2 - r)
    )


def main():
    size = 1024
    out_path = os.path.join(os.path.dirname(__file__), "..", "assets", "icon.png")
    out_path = os.path.abspath(out_path)
    os.makedirs(os.path.dirname(out_path), exist_ok=True)

    # 背景渐变：#4F7CFF -> #7C5CFF
    top = (0x4F, 0x7C, 0xFF)
    bottom = (0x7C, 0x5C, 0xFF)

    bg = rounded_rect_alpha(64, 64, size - 128, size - 128, 208)
    bars = [
        rounded_rect_alpha(256, 300, 512, 96, 48),
        rounded_rect_alpha(256, 468, 384, 96, 48),
        rounded_rect_alpha(256, 636, 448, 96, 48),
    ]

    raw = bytearray()
    for py in range(size):
        raw.append(0)  # filter type 0
        t = py / (size - 1)
        for px in range(size):
            if bg(px, py):
                r = int(top[0] + (bottom[0] - top[0]) * t)
                g = int(top[1] + (bottom[1] - top[1]) * t)
                b = int(top[2] + (bottom[2] - top[2]) * t)
                a = 255
            else:
                r = g = b = a = 0
            if a == 255 and any(bar(px, py) for bar in bars):
                r = g = b = 255
            raw += bytes((r, g, b, a))

    def chunk(tag, data):
        c = struct.pack(">I", len(data)) + tag + data
        c += struct.pack(">I", zlib.crc32(tag + data) & 0xFFFFFFFF)
        return c

    png = b"\x89PNG\r\n\x1a\n"
    png += chunk(b"IHDR", struct.pack(">IIBBBBB", size, size, 8, 6, 0, 0, 0))
    png += chunk(b"IDAT", zlib.compress(bytes(raw), 9))
    png += chunk(b"IEND", b"")

    with open(out_path, "wb") as f:
        f.write(png)
    print("icon written:", out_path, len(png), "bytes")


if __name__ == "__main__":
    main()
