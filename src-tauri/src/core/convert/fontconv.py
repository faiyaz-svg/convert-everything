"""Font conversion with fontTools. Usage: fontconv.py INPUT OUTPUT FORMAT

FORMAT is ttf, otf, woff or woff2. WOFF and WOFF2 keep the outlines they are
given. TTF needs quadratic (TrueType) outlines and OTF needs cubic (CFF)
outlines, so those convert the outlines when needed.
"""

import sys

from fontTools.ttLib import TTFont, newTable


def cff_to_glyf(font):
    from fontTools.pens.cu2quPen import Cu2QuPen
    from fontTools.pens.ttGlyphPen import TTGlyphPen

    order = font.getGlyphOrder()
    glyph_set = font.getGlyphSet()
    glyf = newTable("glyf")
    glyf.glyphOrder = order
    glyf.glyphs = {}
    for name in order:
        pen = TTGlyphPen(glyph_set)
        glyph_set[name].draw(Cu2QuPen(pen, max_err=1.0, reverse_direction=True))
        glyf[name] = pen.glyph()
    font["glyf"] = glyf
    font["loca"] = newTable("loca")
    for tag in ("CFF ", "CFF2", "VORG"):
        if tag in font:
            del font[tag]

    maxp = newTable("maxp")
    maxp.tableVersion = 0x00010000
    maxp.numGlyphs = len(order)
    for attr in ("maxZones",):
        setattr(maxp, attr, 1)
    for attr in (
        "maxPoints", "maxContours", "maxCompositePoints", "maxCompositeContours",
        "maxTwilightPoints", "maxStorage", "maxFunctionDefs", "maxInstructionDefs",
        "maxStackElements", "maxSizeOfInstructions", "maxComponentElements",
        "maxComponentDepth",
    ):
        setattr(maxp, attr, 0)
    font["maxp"] = maxp

    post = font["post"]
    post.formatType = 2.0
    post.extraNames = []
    post.mapping = {}
    post.glyphOrder = order
    font["head"].glyphDataFormat = 0
    font.sfntVersion = "\x00\x01\x00\x00"


def glyf_to_cff(font):
    from fontTools.fontBuilder import FontBuilder
    from fontTools.pens.t2CharStringPen import T2CharStringPen

    order = font.getGlyphOrder()
    glyph_set = font.getGlyphSet()
    hmtx = font["hmtx"]
    charstrings = {}
    for name in order:
        pen = T2CharStringPen(hmtx[name][0], glyph_set)
        glyph_set[name].draw(pen)
        charstrings[name] = pen.getCharString()

    ps_name = font["name"].getDebugName(6) or "Font"
    builder = FontBuilder(font["head"].unitsPerEm, isTTF=False)
    builder.setupGlyphOrder(order)
    builder.setupCFF(ps_name, {"FullName": font["name"].getDebugName(4) or ps_name}, charstrings, {})
    font["CFF "] = builder.font["CFF "]

    for tag in ("glyf", "loca", "fpgm", "prep", "cvt ", "gasp", "hdmx", "LTSH", "VDMX", "cvar", "gvar"):
        if tag in font:
            del font[tag]
    maxp = newTable("maxp")
    maxp.tableVersion = 0x00005000
    maxp.numGlyphs = len(order)
    font["maxp"] = maxp
    font["post"].formatType = 3.0
    font.sfntVersion = "OTTO"


def main():
    src, dst, fmt = sys.argv[1], sys.argv[2], sys.argv[3]
    font = TTFont(src)
    is_cff = "CFF " in font or "CFF2" in font
    if fmt == "ttf" and is_cff:
        cff_to_glyf(font)
    elif fmt == "otf" and not is_cff:
        if "gvar" in font:
            print("This is a variable font. Only its default style is kept.", file=sys.stderr)
        glyf_to_cff(font)
    font.flavor = fmt if fmt in ("woff", "woff2") else None
    font.save(dst)


if __name__ == "__main__":
    main()
