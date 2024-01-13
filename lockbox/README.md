# Liquidity Lockbox

## Introduction
This repository contain the liquidity lockbox v1 set of contracts on Solana.

The lockbox program is designed to allow "bonders" to deposit concentrated liquidity tokens (NFTs) from Orca whirlpool
contracts and receive fungible token equivalents. To make this work, only LP NFTs representing a full range can be
deposited for fungible tokens. The fungible tokens can then be transferred to Ethereum mainnet in order to participate
in OLAS bonding programmes.

A more detailed description of the program can be found here:
[Liquidity lockbox concept](https://github.com/valory-xyz/lockbox-solana/blob/main/lockbox/doc/Bonding_mechanism_with_liquidity_on_Solana_v1_v2.pdf).


## Pre-requisites
Make sure you have the versions as mentioned in the main [README](https://github.com/valory-xyz/lockbox-solana/blob/main/README.md)

```
anchor --version
anchor-cli 0.26.0
solana --version
solana-cli 1.14.29 (src:36af529e; feat:139196142)
rustc --version
rustc 1.62.0 (a8314ef7d 2022-06-27)
```


## Development

Install the dependencies:
```
yarn
```

If you need to remove / check dependencies, run:
```
cargo clean
cargo tree
```

You might also want to completely remove the `Cargo.lock` file.

Build the code with:
```
anchor build
```

Run the validator in a separate window:
```
./validator.sh
```

Export environment variables:
```
export ANCHOR_PROVIDER_URL=http://127.0.0.1:8899
export ANCHOR_WALLET=artifacts/id.json
```

To run the initial script that would just initialize the lockbox program along with having Orca Whirlpool program
and required user accounts setup, run:
```
solana airdrop 10000 9fit3w7t6FHATDaZWotpWqN7NpqgL3Lm1hqUop4hAy8h --url localhost && npx ts-node tests/lockbox_init.ts
```

To run integration test, make sure to stop and start the `validator.sh` in a separate window. Then run:
```
solana airdrop 10000 9fit3w7t6FHATDaZWotpWqN7NpqgL3Lm1hqUop4hAy8h --url localhost && npx ts-node tests/liquidity_lockbox.ts
```

The deployed program ID must be `7ahQGWysExobjeZ91RTsNqTCN3kWyHGZ43ud2vB7VVoZ` and corresponds to the `declare_id`
in the `programs/liquidity_lockbox/src/lib.rs` and `Anchor.toml` file.

For debugging a program address, after the launch of local validator, run:
```
solana logs -v --url localhost 7ahQGWysExobjeZ91RTsNqTCN3kWyHGZ43ud2vB7VVoZ
```

## Documents
All the project-related documents are located here: [docs](https://github.com/valory-xyz/lockbox-solana/blob/main/lockbox/doc).

## Tests 

Stub test with only initializing the lockbox program along with having Orca Whirlpool program and required user accounts setup is located here: [lockbox_init.ts](https://github.com/valory-xyz/lockbox-solana/blob/main/lockbox/tests/lockbox_init.ts)

Integration test is located here: [liquidity_lockbox.ts](https://github.com/valory-xyz/lockbox-solana/blob/main/lockbox/tests/liquidity_lockbox.ts) 


## Audits
- The audit is provided as development matures. The latest audit report can be found here: [audits](https://github.com/valory-xyz/lockbox-solana/blob/main/lockbox/audits).
- The list of known vulnerabilities can be found here: [Vulnerabilities list](https://github.com/valory-xyz/lockbox-solana/blob/main/lockbox/doc/Vulnerabilities_list_solana_lockbox_v1.pdf).

## Acknowledgements
The liquidity lockbox contracts were inspired and based on the following sources:
- [Orca](https://github.com/orca-so/whirlpools);
- [EverlastingsongSolsandbox](https://github.com/everlastingsong/solsandbox);
- [Everlastingsong Microscope](https://everlastingsong.github.io/account-microscope);
- [Everlastingsong Nebula](https://everlastingsong.github.io/nebula/).
