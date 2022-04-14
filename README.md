# Include Graph

Tool for generating include graphs over C/C++ projects.

## Usage

```bash
cargo run -- </path/to/project> -c <path/to/compile_commands.json>
```

This tool needs the compilation commands for the project to find all source files and include directories.
For cmake project they can be generated using the `-DCMAKE_EXPORT_COMPILE_COMMANDS=yes` flag.

The tool creates a `dependencies.json` file in the same directory of the `compile_commands.json`, containing all source files and their included headers.
All includes, pointing to headers outside the project's directory are ignored.
