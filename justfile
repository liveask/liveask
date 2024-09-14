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
