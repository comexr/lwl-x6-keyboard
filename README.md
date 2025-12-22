lwl-driver are needed you can find them here: https://comexr.github.io/ 

To install the app you firts need Rust installed and Cargo:

```bash
curl https://sh.rustup.rs -sSf | sh -s -- -y
```
Install Cargo with the default package manager:
```bash
sudo bash -c 'if command -v apt &>/dev/null; then apt update && apt install -y cargo git; elif command -v dnf &>/dev/null; then dnf install -y cargo git; elif command -v pacman &>/dev/null; then pacman -Syu cargo git; elif command -v zypper &>/dev/null; then zypper install -y cargo git; else echo "Unsupported package manager"; exit 1; fi'
```
Then you need to clone the repo:
```bash
git clone https://github.com/comexr/LWL-TONGFANG-Keyboard-LED-controller && cd LWL-TONGFANG-Keyboard-LED-controller
```
As last you need to install it with:
```bash
make install
```
To build it for debian execute:
```bash
cargo deb
```
To build for rpm execute:
```bash
rpmbuild -ba target/release/rpmbuild/SPECS/keyboard-controller.spec \
  -D "_topdir $(pwd)/target/release/rpmbuild" \
  -D "_tmppath $(pwd)target/release/rpmbuild/tmp"
```
You can find the app under the name ```TF Keyboard controller``` or you can run it via the terminal with the command``` keyboard-controller ```
