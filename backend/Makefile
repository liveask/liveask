# makefile backend

URL=http://127.0.0.1:8090

test-create-event:
	echo '{"moderatorEmail": "foo@bar.com", "eventData": {"maxLikes":0,"name":"ev name","description":"desc","shortUrl":"short"}}' | xh POST "${URL}/api/event/add" content-type:application/json

docker-compose:
	docker compose -f docker-compose.local.yml up

run:
	cargo make --env-file=env/local.env run

check:
	cargo make checks
	cargo make clippy-nightly

test-payment:
	xh "${URL}/api/mod/event/upgrade/eventexample/eventexamplemod"

test-version:
	xh "${URL}/api/version"

header-test:
	xh "${URL}/api/event/eventexample" my-header:foobar
