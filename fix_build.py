#!/usr/bin/env python3
"""
Convert UTF-16 file to UTF-8 without corrupting syntax.
Uses exact byte-by-byte conversion.
"""

from pathlib import Path

FILE = "src/procedural/mod.rs"

def convert_utf16_to_utf8(path):
    """Read UTF-16 file and write as UTF-8 preserving all characters."""
    with open(path, 'rb') as f:
        raw = f.read()
    
    # Check if it's UTF-16 LE
    if raw.startswith(b'\xff\xfe'):
        # Decode as UTF-16 LE
        content = raw.decode('utf-16-le')
    else:
        print(f"{path} is not UTF-16 LE, skipping")
        return False
    
    # Write as UTF-8 without BOM
    with open(path, 'w', encoding='utf-8', newline='\n') as f:
        f.write(content)
    
    print(f"Converted {path} from UTF-16 to UTF-8")
    return True


def main():
    path = Path(FILE)
    if not path.exists():
        print(f"{path} not found!")
        return
    
    # Check if backup exists and is different
    bak_path = Path(FILE + ".bak")
    if bak_path.exists():
        print(f"Backup exists at {bak_path}")
    
    convert_utf16_to_utf8(path)
    
    print("\n✅ Done! Now run:")
    print("   git add -A")
    print('   git commit -m "fix: convert mod.rs from UTF-16 to UTF-8"')
    print("   git push")


if __name__ == "__main__":
    main()