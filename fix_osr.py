with open("tests/osr_function_test.rs", "r") as f:
    c = f.read()

import re
c = re.sub(
    r'return;\n\n\n\s*\.block_to_qjs_offset',
    'return;',
    c, flags=re.DOTALL
)

c = re.sub(
    r'let target_block = map[^;]+;',
    'let target_block = 0;',
    c
)

c = c.replace("    return;\n\n\n", "")
c = c.replace(
    "        .iter()\n        .find(|(_, &v)| v == 4)\n        .map(|(&k, _)| k)\n        .unwrap_or(0);",
    ""
)

with open("tests/osr_function_test.rs", "w") as f:
    f.write(c)
