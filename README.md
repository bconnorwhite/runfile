# runfile

A self-documenting task runner for shell scripts with a simple, readable syntax.

## Runfile Syntax

### Commands and Aliases
```runfile
# Simple command
hello:
  echo "Hello, World!"

# Command with aliases
b, build:
  cargo build

# Multiple aliases
t, test, check:
  cargo test
```

### Arguments
```runfile
# Required argument
deploy target:
  echo "Deploying to $target"

# Optional argument
greet name?:
  echo "Hello, ${name:-World}!"

# Varargs
test ...args:
  cargo test $args
```

### Flags
```runfile
# Boolean flags
build --debug --release:
  cargo build $debug $release

# Value flags
build --output=<file>:
  cargo build --output $OUTPUT

# Short and long flags
build -r, --release:
  cargo build $release
```

**Flag Variables:**

Both the value passed to a flag and the flag itself are provided to each command's script.
This makes it easy to either use that value or forward the flag to another script.
- `$flag` - The actual flag/value pair passed (e.g., "--release", "--output=build/app").
- `$FLAG` - The flag's value (e.g., `true`, "build/app")

When a flag's long name contains dashes, variable names use underscores instead.
For example, `--per-crate` becomes `$per_crate` and `$PER_CRATE`.
This only affects the variable names; the forwarded flag string keeps dashes:

```runfile
build --per-crate --mode=<name>:
  # Boolean flag
  echo "$per_crate"   # prints "--per-crate"
  echo "$PER_CRATE"   # prints "true"

  # Value flag example
  echo "$mode"        # prints "--mode=fast" (if --mode=fast was passed)
  echo "$MODE"        # prints "fast"
```

### Groups
```runfile
# ---
# Build
# ---

build:
  cargo build

# ---
# Test
# ---

test:
  cargo test
```

### Multi-line Scripts
```runfile
deploy:
  echo "Building..."
  cargo build --release
  echo "Deploying..."
  ./deploy.sh
```

## Usage

```bash
# List available commands
run

# Run a command
run build

# With arguments
run deploy production

# With flags
run build --release

# Using aliases
run b
```

See `./Runfile` for a complete example.
