# The PC display cards' fonts

The character fonts of IBM's display cards, extracted from ROM images by
`extract.py`. Only the fonts are kept here, not the ROMs. The models load
them as the hardware has them:
- the MDA/CGA character ROM is assembled from them in its own layout;
- the EGA's fonts go into plane 2, as its BIOS does at a mode set.

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

A character's rows are consecutive bytes, the most significant bit
leftmost: 14 bytes a character in the 14-row fonts, 8 in the 8-row ones.
`ega_9x14_alt.bin` is a list of a character code followed by its 14 rows,
ending with a zero byte.

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
