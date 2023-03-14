# pytest-explorer
Fast terminal ui pytest explorer.

![image](https://user-images.githubusercontent.com/23196976/224422665-924eb367-38d4-41e4-8c11-ef1c39a4c73d.png)

## Bench test collecting on aiohttp tests

repo: https://github.com/aio-libs/aiohttp

### with pytest, coverage was turned off

command: `time pytest --co`

`pytest --co  1.21s user 0.06s system 99% cpu 1.278 total`

### with pytexp 

command: `time pytexp -c`

`pytexp -c  0.08s user 0.01s system 83% cpu 0.104 total`

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
Test collecting:
- parametrized tests are not implemented
- inheritanced tests in classes are not implemented

Deep cross-file ast analysis is needed
