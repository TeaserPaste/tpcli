# TeaserPaste CLI (tpc)

**TeaserPaste CLI** (`tpc`) is a powerful command-line interface for interacting with TeaserPaste. It allows you to create, view, manage, and even execute code snippets directly from your terminal.

## Features

- **Manage Snippets**: Create, view, update, delete, and restore snippets.
- **Execute Code**: Run snippets locally in various languages (Python, JS, Rust, Go, etc.) with `tpc run`.
- **Dependency Management**: Automatically detects and installs dependencies for supported languages when running snippets.
- **Secure**: Manages API tokens securely using the system keyring.
- **Developer Friendly**: Syntax highlighting, clipboard integration, and interactive modes.

## Installation

### Method 1: Pre-built Binaries (Recommended)

You can install `tpc` using the automated installer script (powered by [cargo-dist](https://github.com/axodotdev/cargo-dist)):

**Linux & macOS:**
```sh
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/TeaserPaste/tpcli/releases/latest/download/tpc-installer.sh | sh
```

**Windows (PowerShell):**

```powershell
powershell -ExecutionPolicy Bypass -c "irm https://github.com/TeaserPaste/tpcli/releases/latest/download/tpc-installer.ps1 | iex"
```

### Method 2: Cargo

If you have Rust installed, you can install the CLI directly from Crates.io:

```sh
cargo install tpc
```

*Note: Ensure `~/.cargo/bin` is in your PATH.*

## Configuration

Before creating or managing private snippets, you need to authenticate.

1. Get your API Token from your TeaserPaste account settings.
2. Run the following command:

```sh
tpc config set token priv_YOUR_API_TOKEN
```

The token is stored securely in your operating system's keychain/keyring.

## Usage

Here are some common examples of how to use `tpc`.

### 1. Create a Snippet

**Interactive Mode (Easiest):**

```sh
tpc create --interactive
# or simply
tpc create -i

```

**From a File:**

```sh
tpc create --file ./script.py --title "My Python Script" --visibility public
```

**From Stdin:**

```sh
echo "console.log('Hello World')" | tpc create --title "Hello" --language javascript
```

### 2. View a Snippet

View a snippet with syntax highlighting:

```sh
tpc view <SNIPPET_ID>
```

Copy content directly to clipboard:

```sh
tpc view <SNIPPET_ID> --copy
```

### 3. Run a Snippet 🏃

You can fetch and execute a snippet locally. `tpc` supports Python, Node.js, Rust, Go, C++, and more.

```sh
tpc run <SNIPPET_ID>
```

**Advanced Run Options:**

* `--install-deps`: Auto-install detected imports (e.g., npm packages, pip modules) into a temporary environment.
* `--env KEY=VALUE`: Pass environment variables.
* `--on-error-paste`: If the script fails, automatically upload the error log as a private snippet.

```sh
tpc run <SNIPPET_ID> --install-deps --env DEBUG=true
```

### 4. Search Snippets

Search for public snippets by query:

```sh
tpc search "rust web server" --limit 5
```

### 5. Other Commands

* **Clone to file:** `tpc clone <ID>`
* **Fork/Copy:** `tpc copy <ID>`
* **Statistics:** `tpc stats`
* **Delete:** `tpc delete <ID>`

## Help

To see the full list of commands and options, run:

```sh
tpc --help
```

## License

This project is licensed under the MIT or Apache-2.0 license.
