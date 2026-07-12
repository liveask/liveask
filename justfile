default:
    just --list

run-server:
    cd backend && just run

run-client:
    cd frontend && just serve

run:
    parallelrun --kill-others \
        "just run-server" \
        "just run-client"

check-e2e-playwright:
    cd e2e-playwright && just check

check:
    cd backend && just check
    cd frontend && just check
    cd e2e-playwright && just check

sort:
    cargo sort --workspace
