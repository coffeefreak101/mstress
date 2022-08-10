# mstress
A tool for stress testing the NATSSync system

## Building
Build the service using cargo:
```shell
cargo build
```
Build the service as a container image
```shell
make image
```

## Running the service
The service is designed to run in the same environment as the NATSSync server,
and requires access to the mongodb and NATS instances used by the server.

Use environment variables to specify the mongodb and NATS URLs:

| env var | default value |
|---------|---------------|
| NATS_URL | nats://nats:4222
| MONGO_URL | mongodb://mongo

## API
The service has an HTTP API (TCP port 8080) for running the tests.

| Method | Route                  | Description |
|--------|------------------------|-------------|
| GET    | /                      | Returns "Hello, world!" if the service is up
| GET    | /clients               | Returns an array of clients found in the database
| GET    | /tests/mps             | Do a 10 second test for Messages Per Second on all clients
| GET    | /tests/{client_id}     | Test that a client is alive, and available
| GET    | /tests/{client_id}/mps | Do a 10 second MPS test on a specific client