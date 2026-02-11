s = "│             >>> 風采裝備倉庫告示 <<<         │"
for ch in s:
    if ord(ch) > 0x80:
        print(f"Char: '{ch}' (U+{ord(ch):04X})")

s2 = "└───────────────────────┘"
for ch in s2:
    if ord(ch) > 0x80:
        print(f"Char: '{ch}' (U+{ord(ch):04X})")
