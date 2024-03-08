![liveask readme header](/assets/readme_header.png)
[![CI](https://github.com/liveask/liveask/actions/workflows/push.yml/badge.svg)](https://github.com/liveask/liveask/actions/workflows/push.yml)  [![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)  [![made-with-rust](https://img.shields.io/badge/Made%20with-Rust-1f425f.svg)](https://www.rust-lang.org/)

# Contributions
For contributions please makes changes on a local development version, make sure the changes pass all the tests & do not break anything then, create a pull request.

# Local Setup
**Requires Three Terminal Tabs/Instances**
## Initial Setup
### Pre-requisites 
```
How to install Rust
https://www.rust-lang.org/tools/install

How to install Docker
https://docs.docker.com/engine/install

How to install Make
sudo apt-get install build-essential
```

### Installation
```
Commands To Run
rustup update
rustup target add wasm32-unknown-unknown
cargo install cargo-make
git clone https://github.com/liveask/liveask.git
cd liveask
```

## Running The Local Instance
### Dependencies
**First Terminal**
This is required to run up all dependencies for the application 
```
cd backend
make docker-compose
```
### Backend
**Second Terminal**
This will load up the backend and connect to the dependencies
```
cd backend
make run
```
### Front End
**Third Terminal**
The to load the frontend
```
cd frontend
make serve
```
## Configuration
To configure the application copy the default.env to local.env & edit local.env
```
cd backend/env
cp default.env local.env
```
## Notes
- When doing local development set `RELAX_CORS` to `"1"` otherwise the backend will not get requests
- Do not commit the index.html if only the release id has changed.
