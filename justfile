# https://just.systems

clear:
    clear

default:
    just --choose

outdated: clear
    cargo outdated --workspace

test: clear
    cargo test --workspace

machete: clear
    cargo machete
scc: clear
    scc .
