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

```bash
sudo apt-get install build-essential
```

Install `parallelrun` to run commands in parallel:

```
cargo install parallelrun
```

Install `just` to run project specific commands:

```
cargo install just
```

### Installation

Run the following commands to finish the installation:

```
rustup update
rustup target add wasm32-unknown-unknown
cargo install cargo-make
git clone https://github.com/liveask/liveask.git
cd liveask
```

## Running The Local Instance

> [!IMPORTANT]
> You need to open three terminal tabs/instances and run the following commands in each of them.

### Terminal 1: Dependencies
This is required to run up all dependencies for the application
```
cd backend
just docker-compose
```

### Terminal 2: Backend
This will load up the backend and connect to the dependencies
```
cd backend
just run
```

### Terminal 3: Front End
Then to load the frontend
```
cd frontend
just serve
```

## Configuration
To configure the application copy the `default.env` to `local.env` & edit `local.env`
```
cd backend/env
cp default.env local.env
```

## Notes
- When doing local development set `RELAX_CORS` to `"1"` otherwise the backend will not get requests
- Do not commit the `index.html` if only the release id has changed.
