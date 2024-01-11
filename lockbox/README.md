# Liquidity Lockbox

## Introduction
This repository contain the liquidity lockbox set of contracts on Solana.

The lockbox program is designed to allow "bonders" to deposit concentrated liquidity tokens (NFTs) from Orca whirlpool
contracts and receive fungible token equivalents. To make this work, only LP NFTs representing a full range can be
deposited for fungible tokens. The fungible tokens can then be transferred to Ethereum mainnet in order to participate
in OLAS bonding programmes.

The description of the concept can be found here:
[Liquidity lockbox concept](https://github.com/valory-xyz/lockbox-solana/blob/main/lockbox/docs/Bonding_mechanism_with_liquidity_on_Solana?raw=true).


## Pre-requisites
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

Then, execute the testing script:
```
solana airdrop 10000 9fit3w7t6FHATDaZWotpWqN7NpqgL3Lm1hqUop4hAy8h --url localhost && npx ts-node tests/liquidity_lockbox.ts
```

If the `@programId` in lib.rs does not match with the deployed one, update it and re-run
```
anchor build
```

For debugging, after run local validator:
```
solana logs -v --url localhost 7ahQGWysExobjeZ91RTsNqTCN3kWyHGZ43ud2vB7VVoZ
```

### Audits
The audit is provided as development matures. The latest audit report can be found here: [audits](https://github.com/valory-xyz/lockbox-solana/blob/main/lockbox/audits).

## Acknowledgements
The liquidity lockbox contracts were inspired and based on the following sources:
- [Orca](https://github.com/orca-so/whirlpools);
- [EverlastingsongSolsandbox](https://github.com/everlastingsong/solsandbox);
- [Everlastingsong Microscope](https://everlastingsong.github.io/account-microscope);
- [Everlastingsong Nebula](https://everlastingsong.github.io/nebula/).
