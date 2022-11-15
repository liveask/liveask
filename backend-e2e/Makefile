URL=http://localhost:8090
EVENT=001SMTRGHBZZB733CR1XTQ9G9D

e2e-test:
	docker compose build
	docker compose up --exit-code-from test

test-create-question:
	echo '{"text": "test"}' | xh POST "${URL}/api/event/addquestion/${EVENT}" content-type:application/json

e2e-test-remote:
	RUST_LOG=debug URL=https://api.www.live-ask.com SOCKET_URL="wss://api.www.live-ask.com" cargo test

e2e-local:
	RUST_LOG=debug cargo test