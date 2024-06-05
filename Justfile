docker-build:
    docker build . --label hanne-is-leuk-bot -t latest

bench-docker-build:
    time docker build . -t hanne-is-leuk-bot:latest
    docker images hanne-is-leuk-bot:latest