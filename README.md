![image](https://user-images.githubusercontent.com/23196976/219866003-04e9a353-b104-4b4b-a20f-f85a9c7961c3.png)

# Build from source
- Install rust (Minimum supported version: 1.67)
https://www.rust-lang.org/tools/install
- `git clone https://github.com/antonguzun/pytest-explorer.git`
- `cd pytest-explorer`
- build with cargo `cargo build --release`
- set softlink `ls -s $(pwd)/target/release/pytexp /usr/bin/pytexp`

# Usage

- activate virtual env `source ./venv/bin/activate`
- set PYTHONPATH if needed
- start pytexp in directory with tests

# Known Limitations

- parametrized tests are not implemented
- inheritanced tests in classes are not implemented

Deep cross-file ast analysis is needed
