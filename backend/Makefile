URL=http://127.0.0.1:8090

test-create-event:
	echo '{"moderatorEmail": "foo@bar.com", "eventData": {"maxLikes":0,"name":"ev name","description":"desc","shortUrl":"short"}}' | xh POST "${URL}/api/addevent" content-type:application/json