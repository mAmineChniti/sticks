# ine

A C++ project created with [sticks](https://github.com/mAmineChniti/sticks).

## Building

### Using CMake (Recommended)
```bash
mkdir build
cd build
cmake ..
cmake --build .
```

### Using Makefile
```bash
make
make run
```

### Using Make with Debug
```bash
make debug
```

## Project Structure

```
ine/
├── src/              # Source files
├── include/          # Header files (if applicable)
├── build/            # Build artifacts (generated)
├── CMakeLists.txt    # CMake configuration
├── Makefile          # Makefile configuration
└── README.md         # This file
```

## Adding Dependencies

### Using sticks
```bash
sticks add libcurl openssl
```

### Adding Source Files
```bash
sticks src utils network
```

## License

This project is licensed under the MIT License.
