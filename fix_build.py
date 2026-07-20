#!/usr/bin/env python3
"""
Convert UTF-16 files to UTF-8
"""

from pathlib import Path

FILES = [
    "src/procedural/mod.rs",
    "src/renderer/dof.rs",
    "src/renderer/parallel.rs"
]


def convert_to_utf8(path):
    """Read file (auto-detect encoding) and write as UTF-8."""
    with open(path, 'rb') as f:
        raw = f.read()
    
    # Auto-detect encoding
    if raw.startswith(b'\xff\xfe'):
        content = raw.decode('utf-16-le')
    elif raw.startswith(b'\xfe\xff'):
        content = raw.decode('utf-16-be')
    elif raw.startswith(b'\xef\xbb\xbf'):
        content = raw.decode('utf-8-sig')
    else:
        content = raw.decode('utf-8')
    
    # Write as UTF-8 without BOM
    with open(path, 'w', encoding='utf-8', newline='\n') as f:
        f.write(content)
    
    print(f"Converted {path}")


def main():
    for f in FILES:
        path = Path(f)
        if path.exists():
            convert_to_utf8(path)
        else:
            print(f"Skipping {f} - not found")
    
    print("\nAll files converted to UTF-8!")
    print("\nNow commit and push:")
    print("   git add -A")
    print('   git commit -m "fix: convert files to UTF-8"')
    print("   git push")


if __name__ == "__main__":
    main()