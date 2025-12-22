pub fn generate_precommit_hook() -> &'static str {
	"#!/bin/bash\n\
    \n\
    set -e\n\
    \n\
    echo \"Running pre-commit checks...\"\n\
    \n\
    # Format check\n\
    echo \"  Checking code formatting...\"\n\
    find src -name '*.c' -o -name '*.cpp' -o -name '*.h' | xargs clang-format --dry-run -i\n\
    \n\
    echo \"  All checks passed!\"\n"
}
