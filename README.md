To install the app you firts need Rust installed and Cargo:

```bash
curl https://sh.rustup.rs -sSf | sh -s -- -y
```
Install Cargo with the default package manager:
```bash
sudo bash -c 'if command -v apt &>/dev/null; then apt update && apt install -y cargo; elif command -v dnf &>/dev/null; then dnf install -y cargo; elif command -v pacman &>/dev/null; then pacman -Syu cargo; elif command -v zypper &>/dev/null; then zypper install -y cargo; else echo "Unsupported package manager"; exit 1; fi'
```
