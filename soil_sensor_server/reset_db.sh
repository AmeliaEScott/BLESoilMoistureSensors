#!/bin/bash

docker compose down
rm -rf postgres-data
mkdir postgres-data
docker compose up -d
sleep 3
diesel setup
diesel migration run
mv src/schema.rs src/database/schema.rs

# psql postgres://postgres:pleasedontstealmypassword@localhost:5432/soil_sensors
