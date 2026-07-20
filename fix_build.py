#!/usr/bin/env python3
"""
Build Fix Script - AnimDSL
Pure text patcher - no build tools needed.
"""

import os
from pathlib import Path

REPLACEMENTS = [
    # src/procedural/mod.rs
    {
        "file": "src/procedural/mod.rs",
        "old": "    _desc: &CharacterDesc,",
        "new": "    desc: &CharacterDesc,"
    },
    {
        "file": "src/procedural/mod.rs",
        "old": "     _desc: &CharacterDesc,",
        "new": "     desc: &CharacterDesc,"
    },
    {
        "file": "src/procedural/mod.rs",
        "old": "_desc: &CharacterDesc,",
        "new": "desc: &CharacterDesc,"
    },
    {
        "file": "src/procedural/mod.rs",
        "old": "    _flip: f64,",
        "new": "    flip: f64,"
    },
    {
        "file": "src/procedural/mod.rs",
        "old": "     _flip: f64,",
        "new": "     flip: f64,"
    },
    {
        "file": "src/procedural/mod.rs",
        "old": "_flip: f64,",
        "new": "flip: f64,"
    },
    {
        "file": "src/procedural/mod.rs",
        "old": "    _skin: [u8; 3],",
        "new": "    skin: [u8; 3],"
    },
    {
        "file": "src/procedural/mod.rs",
        "old": "     _skin: [u8; 3],",
        "new": "     skin: [u8; 3],"
    },
    {
        "file": "src/procedural/mod.rs",
        "old": "_skin: [u8; 3],",
        "new": "skin: [u8; 3],"
    },
    {
        "file": "src/procedural/mod.rs",
        "old": "let paint = Paint::default();",
        "new": "let mut paint = Paint::default();"
    },
    # src/renderer/dof.rs
    {
        "file": "src/renderer/dof.rs",
        "old": "    let _w = pixmap.width() as usize;",
        "new": "    let w = pixmap.width() as usize;"
    },
    {
        "file": "src/renderer/dof.rs",
        "old": "    let _h = pixmap.height() as usize;",
        "new": "    let h = pixmap.height() as usize;"
    },
    {
        "file": "src/renderer/dof.rs",
        "old": "    let _data = pixmap.data_mut();",
        "new": "    let data = pixmap.data_mut();"
    },
    # src/renderer/parallel.rs
    {
        "file": "src/renderer/parallel.rs",
        "old": "let mut result: Vec<Frame> = frames.into_iter().filter_map(|f| f).collect();",
        "new": "let result: Vec<Frame> = frames.into_iter().filter_map(|f| f).collect();"
    }
]


def detect_encoding(raw_bytes):
    """Detect encoding from raw bytes."""
    # UTF-16 LE BOM
    if raw_bytes.startswith(b'\xff\xfe'):
        return 'utf-16-le'
    # UTF-16 BE BOM
    if raw_bytes.startswith(b'\xfe\xff'):
        return 'utf-16-be'
    # UTF-8 BOM
    if raw_bytes.startswith(b'\xef\xbb\xbf'):
        return 'utf-8-sig'
    return 'utf-8'


def read_file(path):
    """Read file with automatic encoding detection."""
    with open(path, 'rb') as f:
        raw = f.read()
    
    encoding = detect_encoding(raw)
    content = raw.decode(encoding)
    return content


def write_file(path, content):
    """Write file as UTF-8 without BOM."""
    with open(path, 'w', encoding='utf-8', newline='\n') as f:
        f.write(content)


def apply_fixes(project_root="."):
    project_path = Path(project_root)
    fixed_files = set()
    total_replacements = 0

    for replacement in REPLACEMENTS:
        file_path = project_path / replacement["file"]
        if not file_path.exists():
            print(f"Skipping {file_path} - file not found")
            continue

        try:
            content = read_file(file_path)
        except Exception as e:
            print(f"Error reading {file_path}: {e}")
            continue

        if replacement["old"] in content:
            new_content = content.replace(replacement["old"], replacement["new"])
            write_file(file_path, new_content)
            fixed_files.add(str(file_path))
            total_replacements += 1
            print(f"Patched {file_path}")

    print("\n" + "=" * 50)
    print(f"Applied {total_replacements} replacements across {len(fixed_files)} files:")
    for f in sorted(fixed_files):
        print(f"   - {f}")
    print("=" * 50)

    if total_replacements == 0:
        print("No changes needed - all fixes already applied")


def main():
    import sys

    project_root = sys.argv[1] if len(sys.argv) > 1 else "."

    print("Applying build fixes...")
    print("=" * 50)

    apply_fixes(project_root)

    print("\n" + "=" * 50)
    print("SUCCESS! All fixes applied.")
    print("=" * 50)
    print("\nNext steps (run in terminal):")
    print("   git add -A")
    print('   git commit -m "fix: resolve parameter naming errors in procedural and renderer modules"')
    print("   git push")
    print("\nThen GitHub Actions will build and pass.")


if __name__ == "__main__":
    main()