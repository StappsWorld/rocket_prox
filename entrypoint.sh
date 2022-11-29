#!/bin/bash

if [ ! -f /app/Rocket.toml ]; then
    echo "[release]
address = \"0.0.0.0\"
port = 9999
secret_key = \"$(LC_ALL=C tr -dc 'A-Za-z0-9' </dev/urandom | head -c 88)\"
[default.limits]
forms = \"64 kB\"
json = \"1 MiB\"
msgpack = \"100 MiB\"
\"file/jpg\" = \"100 MiB\"
file = \"100 MiB\"
data-form=\"100 MiB\"" > /app/Rocket.toml
fi

# Run service
/app/rocket_prox