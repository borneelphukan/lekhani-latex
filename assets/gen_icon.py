import struct, zlib

def create_png(width, height):
    def chunk(ctype, data):
        c = ctype + data
        return struct.pack('>I', len(data)) + c + struct.pack('>I', zlib.crc32(c) & 0xffffffff)

    header = b'\x89PNG\r\n\x1a\n'
    ihdr = chunk(b'IHDR', struct.pack('>IIBBBBB', width, height, 8, 2, 0, 0, 0))

    raw = bytearray()
    for y in range(height):
        raw.append(0)
        for x in range(width):
            cx, cy = x - width // 2, y - height // 2
            dist = (cx * cx + cy * cy) ** 0.5
            r = width * 0.4
            if dist < r:
                raw.extend([40, 120, 200])
            else:
                raw.extend([240, 240, 240])

    idat = chunk(b'IDAT', zlib.compress(bytes(raw)))
    iend = chunk(b'IEND', b'')
    return header + ihdr + idat + iend

iconset = "latex-writer.iconset"
sizes = [(16, 16), (32, 32), (128, 128), (256, 256), (512, 512)]
for w, h in sizes:
    data = create_png(w, h)
    with open(f'{iconset}/icon_{w}x{h}.png', 'wb') as f:
        f.write(data)
    data2x = create_png(w * 2, h * 2)
    with open(f'{iconset}/icon_{w}x{h}@2x.png', 'wb') as f:
        f.write(data2x)
