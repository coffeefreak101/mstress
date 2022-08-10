#!/bin/bash

client_id=$1

curl localhost/tests/${client_id} | jq .

