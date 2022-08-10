#!/bin/bash

client_id=$1

curl localhost:8080/tests/${client_id}/mps | jq .

