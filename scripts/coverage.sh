#!/bin/sh
# Initialize variables
OPEN_IN_BROWSER=false

# Parse arguments
for arg in "$@"
do
  if [ "$arg" == "--open" ] || [ "$arg" == "-o" ]; then
    OPEN_IN_BROWSER=true
  fi
done

# Create coverage directory
mkdir -p ./target/llvm-cov/lcov

# Generate LCOV
cargo llvm-cov --lcov --output-path target/llvm-cov/coverage.lcov

# Generate HTML
genhtml ./target/llvm-cov/coverage.lcov --output-directory ./target/llvm-cov/lcov --hierarchical --legend --show-details

# Open in browser
if [ "$OPEN_IN_BROWSER" = true ]; then
  open target/llvm-cov/lcov/index.html
fi
