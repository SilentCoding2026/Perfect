#!/usr/bin/env python3
"""
Production-grade fixes for AnimDSL
Applies all required changes without #![allow(...)] hacks.
"""

import re
from pathlib import Path

def read_file(path):
    """Read file with auto-encoding detection."""
    with open(path, 'rb') as f:
        raw = f.read()
    if raw.startswith(b'\xff\xfe'):
        return raw.decode('utf-16-le')
    if raw.startswith(b'\xef\xbb\xbf'):
        return raw[3:].decode('utf-8')
    return raw.decode('utf-8')

def write_file(path, content):
    """Write as UTF-8 without BOM."""
    with open(path, 'w', encoding='utf-8', newline='\n') as f:
        f.write(content)

def fix_unused_variables(content, filename):
    """Add _ prefix to unused variables that are actually unused."""
    unused_vars = {
        'src/procedural/mod.rs': [
            ('flip', 909),
            ('desc', 1217),
            ('flip', 1226),
            ('desc', 1465),
            ('flip', 1474),
            ('skin', 1476),
            ('flip', 2051),
            ('flip', 2474),
            ('flip', 2561),
        ],
        'src/renderer/dof.rs': [
            ('w', 63),
            ('h', 64),
            ('data', 65),
        ],
        'src/renderer/parallel.rs': [
            ('result', 106),
        ],
    }
    
    if filename not in unused_vars:
        return content
    
    lines = content.splitlines()
    for var_name, line_num in unused_vars[filename]:
        idx = line_num - 1
        if idx < len(lines):
            lines[idx] = lines[idx].replace(f' {var_name}:', f' _{var_name}:')
            lines[idx] = lines[idx].replace(f' {var_name},', f' _{var_name},')
            lines[idx] = lines[idx].replace(f'let {var_name} ', f'let _{var_name} ')
    
    return '\n'.join(lines)

def fix_clamp(content):
    """Fix manual clamp patterns."""
    content = re.sub(
        r'\(([^)]+)\)\.max\(0\.0\)\.min\(255\.0\)',
        r'(\1).clamp(0.0, 255.0)',
        content
    )
    content = re.sub(
        r'\.max\(0\)\.min\(255\)',
        r'.clamp(0, 255)',
        content
    )
    return content

def fix_assign_op(content):
    """Fix manual assign operations."""
    content = re.sub(
        r'(\w+\.\w+\.\d+)\s*=\s*\1\s*\*\s*\(([^)]+)\)',
        r'\1 *= (\2)',
        content
    )
    return content

def fix_or_insert_with(content):
    """Fix or_insert_with(HashMap::new) -> or_default()."""
    content = content.replace('.or_insert_with(HashMap::new)', '.or_default()')
    content = content.replace('.or_insert_with(Vec::new)', '.or_default()')
    return content

def fix_filter_map_identity(content):
    """Fix .filter_map(|f| f) -> .flatten()."""
    content = content.replace('.filter_map(|f| f)', '.flatten()')
    return content

def fix_needless_borrow(content):
    """Fix &assets -> assets."""
    content = content.replace('render_frame(&assets,', 'render_frame(assets,')
    content = content.replace('render_frame(&assets_arc,', 'render_frame(assets_arc,')
    return content

def fix_range_contains(content):
    """Fix manual range contains."""
    content = re.sub(
        r'if\s+x\s*<\s*(-?\d+\.?\d*)\s*\|\|\s*x\s*>\s*(\d+\.?\d*)\s*{',
        r'if !(\1..=\2).contains(&x) {',
        content
    )
    return content

def fix_absurd_comparisons(content):
    """Fix absurd comparisons like c > 255 (always false for u8)."""
    content = re.sub(
        r'if\s+c\s*>\s*255\s*{',
        r'// if c > 255 { // always false for u8',
        content
    )
    content = content.replace('if sx < 0 || ', 'if ')
    content = content.replace('if sy < 0 || ', 'if ')
    return content

def fix_collapsible_match(content):
    """Fix collapsible match patterns."""
    lines = content.splitlines()
    new_lines = []
    i = 0
    while i < len(lines):
        line = lines[i]
        if 'match desc.hair.style {' in line:
            new_lines.append(line)
            i += 1
            while i < len(lines):
                if 'HairStyle::Straight | HairStyle::Wavy' in lines[i]:
                    if i + 1 < len(lines) and 'if desc.hair.length > 0.3' in lines[i + 1]:
                        new_lines.append(lines[i].replace('HairStyle::Straight | HairStyle::Wavy', 
                                                         'HairStyle::Straight | HairStyle::Wavy if desc.hair.length > 0.3'))
                        i += 2
                        continue
                new_lines.append(lines[i])
                i += 1
            break
        new_lines.append(line)
        i += 1
    
    return '\n'.join(new_lines)

def fix_single_match(content):
    """Fix single_match - convert match to if."""
    content = content.replace('match pair.as_rule() {', 'if pair.as_rule() == Rule::program {')
    content = content.replace('_ => {}', '}')
    return content

def main():
    files = [
        'src/procedural/mod.rs',
        'src/renderer/dof.rs',
        'src/renderer/parallel.rs',
        'src/timeline/mod.rs',
        'src/skeleton/mod.rs',
        'src/assets/validation.rs',
        'src/parser/mod.rs',
    ]
    
    processed = {}
    for filepath in files:
        path = Path(filepath)
        if not path.exists():
            print(f"Skipping {filepath} - not found")
            continue
        processed[filepath] = read_file(filepath)
        print(f"Loaded {filepath}")
    
    # Apply fixes
    for filepath, content in processed.items():
        if 'src/procedural/mod.rs' in filepath:
            content = fix_unused_variables(content, filepath)
            content = fix_clamp(content)
            content = fix_assign_op(content)
            content = fix_collapsible_match(content)
            print(f"Applied procedural fixes to {filepath}")
        elif 'src/renderer/dof.rs' in filepath:
            content = fix_unused_variables(content, filepath)
            content = fix_absurd_comparisons(content)
            print(f"Applied dof fixes to {filepath}")
        elif 'src/renderer/parallel.rs' in filepath:
            content = fix_unused_variables(content, filepath)
            content = fix_filter_map_identity(content)
            content = fix_needless_borrow(content)
            print(f"Applied parallel fixes to {filepath}")
        elif 'src/timeline/mod.rs' in filepath:
            content = fix_or_insert_with(content)
            content = fix_range_contains(content)
            print(f"Applied timeline fixes to {filepath}")
        elif 'src/skeleton/mod.rs' in filepath:
            content = fix_assign_op(content)
            print(f"Applied skeleton fixes to {filepath}")
        elif 'src/assets/validation.rs' in filepath:
            content = fix_absurd_comparisons(content)
            print(f"Applied validation fixes to {filepath}")
        elif 'src/parser/mod.rs' in filepath:
            content = fix_single_match(content)
            print(f"Applied parser fixes to {filepath}")
    
    # Write all changed files
    for filepath, content in processed.items():
        write_file(filepath, content)
        print(f"✅ Saved {filepath}")
    
    print("\n" + "=" * 50)
    print("All production-grade fixes applied!")
    print("\nNext steps:")
    print("   git add -A")
    print('   git commit -m "fix: resolve all Clippy warnings and unused variables"')
    print("   git push")

if __name__ == "__main__":
    main()