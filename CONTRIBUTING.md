# Contributions

For contributions please:
1. Make sure your fork (fork it if it's the first time) is in sync with the parent repository.
2. Make your changes in a topic branch in your fork.
2. Make sure the changes pass all the tests & do not break anything.
3. When ready create a pull request from your fork to the parent repository.

# Local Setup

## Initial Setup

### Pre-requisites

1. Install Rust: https://www.rust-lang.org/tools/install
2. Install Docker: https://docs.docker.com/engine/install

Install Make:

```shell
sudo apt-get install build-essential
```

Install `parallelrun` to run commands in parallel:

```shell
cargo install parallelrun
```

> !TIP
> You can also use `cargo binstall parallelrun` if you want to avoid building from source.


Install `just` to run project specific commands:

```shell
cargo install just
```

### Installation

Run the following commands to finish the installation:

```shell
rustup update
rustup target add wasm32-unknown-unknown
cargo install cargo-make
git clone https://github.com/liveask/liveask.git
cd liveask
```

## Running The Local Instance

> [!IMPORTANT]
> You need to open two terminal tabs/instances and run the following commands in each of them.

### Terminal 1: Dependencies
This is required to run up all dependencies for the application
```shell
cd backend
just docker-compose
```

### Terminal 2: Backend & Frontend
This will load up the backend and connect to the dependencies, and also run the frontend (need to run in the root directory of the repository):
```shell
cd ..
just run
```

## Configuration
To configure the application copy the `default.env` to `local.env` & edit `local.env`
```shell
cd backend/env
cp default.env local.env
```

## Notes
- When doing local development set `RELAX_CORS` to `"1"` in `local.env`, otherwise the backend will not get requests
- Do not commit the `index.html` if only the release id has changed.
