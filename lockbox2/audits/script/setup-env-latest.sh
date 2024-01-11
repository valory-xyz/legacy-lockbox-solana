#!/bin/bash
RUSTVER="1.70.0" 
SOLANAVER="1.17.7" 
ANCHORVER="0.29.0" 

### uncomment next line only when installing from scratch (no rustc in OS)
# curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup install $RUSTVER
rustup default $RUSTVER

curl -sSfL https://release.solana.com/v${SOLANAVER}/install | sh

### uncomment next line only when installing from scratch (no anchor in OS), for initial setup needed RUSTVER="1.70.0"
# cargo install --git https://github.com/coral-xyz/anchor avm --locked --force
avm install $ANCHORVER
avm use $ANCHORVER
