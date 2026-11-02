"""Extract the PC display cards' character fonts from ROM images.

    python3 -I extract.py ~/roms

Only the fonts are kept in the repository; the ROM images are not. The
images are those archived at https://minuszerodegrees.net/rom/rom.htm, and
the script refuses any other (it checks their SHA-256), so the fonts can be
extracted again exactly. See README.md for the layouts.
"""
import hashlib
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))

SOURCES = {
    # The character ROM of the IBM MDA and CGA (one part for both cards).
    'IBM_5788005_AM9264_1981_CGA_MDA_CARD.BIN':
        '37527f661580e5a09710051cec67422abfadf31c61b841537a91e7f27be19304',
    # The IBM EGA's BIOS ROM, dumped byte-reversed.
    'ibm_6277356_ega_card_u44_27128.bin':
        'b26fea4da9a3ba4af8eda55a32cfc5a67d62e24be5960c1bbac857e8ef0fb5b2',
    # The IBM VGA's (PS/2 Display Adapter's) BIOS: a 24 KB option ROM at
    # 2000 in a 32 KB image.
    'ibm_vga.bin':
        '51b49761f72780a696c36f3d2e74a8168c1c99b865070f7f90b8e621a8b5ae17',
}

# 'A' in each font, as a check that the offsets are right.
A_THICK = bytes.fromhex('3078cccc fccccc00'.replace(' ', ''))
A_THIN = bytes.fromhex('182442427e424200')
A_MDA = bytes.fromhex('0000081c366363 7f636363000000'.replace(' ', ''))
A_EGA14 = bytes.fromhex('000010386cc6c6fec6c6c6000000')
A_VGA16 = bytes.fromhex('000010386cc6c6fec6c6c6c6000000 00'.replace(' ', ''))


def alternates(rom, at, rows):
    """An alternate-glyph table at `at`: a character code and its rows
    each, ending with a zero; returned with the zero."""
    end = at
    while rom[end]:
        end += 1 + rows
        if end - at > 64 * (1 + rows):
            sys.exit('alternate table at %#x does not end' % at)
    return rom[at:end + 1]


def load(rom_dir, name):
    data = open(os.path.join(rom_dir, name), 'rb').read()
    digest = hashlib.sha256(data).hexdigest()
    if digest != SOURCES[name]:
        sys.exit('%s: unexpected contents (sha256 %s)' % (name, digest))
    return data


def check(what, got, want):
    if got != want:
        sys.exit('%s: expected %s, found %s' % (what, want.hex(), got.hex()))


def main(rom_dir):
    out = {}

    cga = load(rom_dir, 'IBM_5788005_AM9264_1981_CGA_MDA_CARD.BIN')
    # The MDA font: rows 0-7 of each character in the first 2 KB, rows 8-15
    # in the second (rows 14 and 15 are blank throughout).
    mda = bytearray()
    for c in range(256):
        mda += cga[c * 8:c * 8 + 8] + cga[0x800 + c * 8:0x800 + c * 8 + 6]
        assert not any(cga[0x800 + c * 8 + 6:0x800 + c * 8 + 8])
    out['mda_9x14.bin'] = bytes(mda)
    out['cga_8x8_thin.bin'] = cga[0x1000:0x1800]
    out['cga_8x8_thick.bin'] = cga[0x1800:0x2000]
    check('MDA A', out['mda_9x14.bin'][0x41 * 14:0x42 * 14], A_MDA)
    check('CGA thin A', out['cga_8x8_thin.bin'][0x41 * 8:0x42 * 8], A_THIN)
    check('CGA thick A', out['cga_8x8_thick.bin'][0x41 * 8:0x42 * 8], A_THICK)

    ega = load(rom_dir, 'ibm_6277356_ega_card_u44_27128.bin')[::-1]
    if ega[:3] != b'\x55\xaa\x20' or sum(ega) & 0xFF:
        sys.exit('EGA BIOS: not a 16 KB option ROM after reversing')
    out['ega_8x14.bin'] = ega[0x2230:0x2230 + 256 * 14]
    # The alternate glyphs for 9-dot text: a character code and its 14
    # rows each, ending with a zero.
    out['ega_9x14_alt.bin'] = alternates(ega, 0x3030, 14)
    out['ega_8x8.bin'] = ega[0x3160:0x3160 + 256 * 8]
    check('EGA 8x14 A', out['ega_8x14.bin'][0x41 * 14:0x42 * 14], A_EGA14)
    check('EGA 8x8 A', out['ega_8x8.bin'][0x41 * 8:0x42 * 8], A_THICK)

    vga = load(rom_dir, 'ibm_vga.bin')[0x2000:]
    if vga[:3] != b'\x55\xaa\x30' or sum(vga[:0x6000]) & 0xFF:
        sys.exit('VGA BIOS: not a 24 KB option ROM at 2000')
    # The VGA BIOS carries the EGA's 8x8 and 8x14 fonts and 9x14
    # alternates unchanged; only its 8x16 font and 9x16 alternates are new.
    check('VGA 8x8', vga[0x378D:0x378D + 256 * 8], out['ega_8x8.bin'])
    check('VGA 8x14', vga[0x3F8D:0x3F8D + 256 * 14], out['ega_8x14.bin'])
    check('VGA 9x14 alternates', alternates(vga, 0x4D8D, 14), out['ega_9x14_alt.bin'])
    out['vga_8x16.bin'] = vga[0x4EBA:0x4EBA + 256 * 16]
    out['vga_9x16_alt.bin'] = alternates(vga, 0x5EBA, 16)
    check('VGA 8x16 A', out['vga_8x16.bin'][0x41 * 16:0x42 * 16], A_VGA16)

    for name, data in out.items():
        with open(os.path.join(HERE, name), 'wb') as f:
            f.write(data)
        print('%-18s %5d bytes  sha256 %s' % (name, len(data), hashlib.sha256(data).hexdigest()))


if __name__ == '__main__':
    main(os.path.expanduser(sys.argv[1] if len(sys.argv) > 1 else '~/roms'))
