# Generates docs/icon.png — the placeholder app icon used by src-tauri/icons/.
"""Generate a 1024x1024 PNG icon for miniterm.
Dark background #0f1114 with a blue #3d7dff terminal cursor block.
Uses only Python standard library (zlib + struct).
"""
import zlib, struct, sys

def make_png(width, height, pixels):
    """pixels: list of (r, g, b) tuples, row-major."""
    def chunk(tag, data):
        c = zlib.crc32(tag + data) & 0xffffffff
        return struct.pack('>I', len(data)) + tag + data + struct.pack('>I', c)

    ihdr = struct.pack('>IIBBBBB', width, height, 8, 2, 0, 0, 0)
    raw = b''
    for y in range(height):
        raw += b'\x00'  # filter type: None
        for x in range(width):
            r, g, b = pixels[y * width + x]
            raw += bytes([r, g, b])
    compressed = zlib.compress(raw, 9)
    return (
        b'\x89PNG\r\n\x1a\n' +
        chunk(b'IHDR', ihdr) +
        chunk(b'IDAT', compressed) +
        chunk(b'IEND', b'')
    )

W, H = 1024, 1024
BG = (0x0f, 0x11, 0x14)   # #0f1114
FG = (0x3d, 0x7d, 0xff)   # #3d7dff

# Cursor block: ~40% width, ~8% height, centred slightly below centre
cw = int(W * 0.40)
ch = int(H * 0.08)
cx = (W - cw) // 2
cy = int(H * 0.52)

pixels = []
for y in range(H):
    for x in range(W):
        if cx <= x < cx + cw and cy <= y < cy + ch:
            pixels.append(FG)
        else:
            pixels.append(BG)

data = make_png(W, H, pixels)
out = sys.argv[1] if len(sys.argv) > 1 else 'icon.png'
with open(out, 'wb') as f:
    f.write(data)
print(f"Written {len(data)} bytes to {out}")
