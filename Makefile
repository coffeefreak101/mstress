image:
	DOCKER_BUILDKIT=1 docker build -t mstress:latest .

container:
	docker run -d --network natssync-net --name mstress -p 80:8080 -e NATS_URL="nats://nats-cloud:4222" -e MONGO_URL="mongodb://mongo-cloud" mstress

push:
	docker push coffeefreak101/mstress:latest

docker:
	docker run -d -p 8080:8080 --name mstress coffeefreak101/mstress:latest || docker start mstress

mongo:
	docker run -d -p 27017:27017 --name mongo mongo || docker start mongo

nats:
	docker run -d -p 4222:4222 --name nats nats || docker start nats

purge:
	docker stop mstress mongo nats
	docker rm mstress mongo nats
