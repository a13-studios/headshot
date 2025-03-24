# Headshot Package
This is Rust package that lets you process profile picture crops from a directory of images.

## Prerequisites

Make sure you have the following installed on your system:

- **Homebrew**: If you don't have Homebrew installed, you can download it [here](https://brew.sh/).

## Installation

1. Install LLVM using Homebrew:

   ```bash
   brew install llvm
   ```

2. Set the necessary environment variable based on your architecture:

   - For Apple Silicon (M1, M2, etc.):

     ```bash
     export LIBCLANG_PATH=$(brew --prefix llvm)/lib
     ```

   - For Intel Macs:

     ```bash
     export LIBCLANG_PATH=/usr/local/opt/llvm/lib
     ```


## Usage

To use the package, [provide a brief description of how to run your package, e.g., command line usage, function calls, etc.]. 

```bash
# Example command
cp target/release/headshot ~/.local/bin/
headshot --input your-image.jpg --output output-image.jpg
```

## Contributing

We welcome contributions! Please read our [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines on how to contribute.

## License

This project is licensed under the [MIT License](LICENSE).