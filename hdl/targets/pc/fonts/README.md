# The PC display cards' fonts

The character fonts of IBM's display cards, extracted from ROM images by
`extract.py`. Only the fonts are kept here, not the ROMs. The models load
them as the hardware has them:
- the MDA/CGA character ROM is assembled from them in its own layout;
- the EGA's and VGA's fonts go into plane 2, as their BIOSes do at a mode
  set (in the tests their own BIOSes load them).

```
python3 -I extract.py ~/roms
```

The sources are the images archived at
<https://minuszerodegrees.net/rom/rom.htm>, captured from real cards.
`extract.py` accepts only those images (it checks their SHA-256), so the
fonts can always be extracted again exactly.

| File | Source | Where |
| --- | --- | --- |
| `mda_9x14.bin` | IBM 5788005 (MDA/CGA character ROM, 8 KB) | rows 0-7 of each character at 0000, rows 8-13 at 0800 |
| `cga_8x8_thin.bin` | IBM 5788005 | 1000 |
| `cga_8x8_thick.bin` | IBM 5788005 | 1800 |
| `ega_8x14.bin` | IBM 6277356 (EGA BIOS, 16 KB) | 2230 |
| `ega_9x14_alt.bin` | IBM 6277356 | 3030: the alternate glyphs for 9-dot text |
| `ega_8x8.bin` | IBM 6277356 | 3160 |
| `vga_8x16.bin` | IBM VGA BIOS (`ibm_vga.bin`: a 24 KB option ROM at 2000 in the 32 KB image) | 4EBA in the option ROM |
| `vga_9x16_alt.bin` | IBM VGA BIOS | 5EBA: the alternate glyphs for 9-dot text |

A character's rows are consecutive bytes, the most significant bit
leftmost: 16, 14 or 8 bytes a character. The alternate tables
(`*_alt.bin`) are lists of a character code followed by its rows (14 or
16), ending with a zero byte.

The VGA BIOS carries the EGA's 8x8 and 8x14 fonts and 9x14 alternates
unchanged (at 378D, 3F8D and 4D8D). `extract.py` checks that they are
identical and doesn't write them twice.

## About the ROMs

- **The 5788005** is one part shared by the MDA and the CGA, which each
  read their own part of it.
  - The MDA's 9x14 font is split in two: the first 2 KB hold rows 0-7 of
    every character and the next 2 KB rows 8-15. Rows 14 and 15 are blank
    throughout.
  - The CGA has two 8x8 fonts, thin at 1000 and thick at 1800. A jumper on
    the card picks one, and thick is the default.
  - The Hercules Graphics Card's character ROM holds the MDA's font.
- **The EGA BIOS image** (6277356, from socket U44) is byte-reversed as
  archived: the last byte of the file is the ROM's first. Its copyright
  notice reads backwards at the end of the file. Reversed, it is a 16 KB
  option ROM: the 55 AA signature, a size byte of 20, and bytes that sum
  to zero.
