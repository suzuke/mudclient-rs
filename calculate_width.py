def get_width(s, box_width=2):
    w = 0
    for ch in s:
        if ch == '\r' or ch == '\n':
            continue
        if 0x2500 <= ord(ch) <= 0x257F:
            w += box_width
        elif ord(ch) > 0x80:
            w += 2
        else:
            w += 1
    return w

lines = [
    "\r        │             >>> 風采裝備倉庫告示 <<<         │",
    "\r        │                                              │         ",
    "\r        │ 列出儲存裝備:  eqstock list                  │  ",
    "\r        │ 儲存指定裝備:  eqstock save 物品名稱         │",
    "\r        │ 領取指定裝備:  eqstock load 物品名稱 或 編號 │",
    "\r        └───────────────────────┘"
]

print("Width Calculation (Box Width = 1):")
for i, line in enumerate(lines):
    print(f"Line {i+1}: {get_width(line, 1)} units")

print("\nWidth Calculation (Box Width = 2):")
for i, line in enumerate(lines):
    print(f"Line {i+1}: {get_width(line, 2)} units")
