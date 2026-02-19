# https://just.systems
set dotenv-load


clear:
    clear

default:
    just --choose

outdated: clear
    cargo outdated --workspace

test: clear
    cargo test --workspace
