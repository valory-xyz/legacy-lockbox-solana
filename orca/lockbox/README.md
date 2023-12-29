# CPI Whirlpool
Set of contracts to call various Orca Whirlpool functions via the CPI.

## Pre-requisites
- Solana version: `solana-cli 1.17.7 (src:fca44b78; feat:3073089885, client:SolanaLabs)`;
- Anchor version: `anchor-cli 0.29.0`.

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

NOTE: These versions actually guarantee tests to work correctly.
```
anchor --version
anchor-cli 0.26.0
solana --version
solana-cli 1.14.29 (src:36af529e; feat:139196142)
rustc --version
rustc 1.62.0 (a8314ef7d 2022-06-27)
```

## Acknowledgements
The liquidity lockbox contracts were inspired and based on the following sources:
- [Orca](https://github.com/orca-so/whirlpools);
- [EverlastingsongSolsandbox](https://github.com/everlastingsong/solsandbox);
- [Everlastingsong Microscope](https://everlastingsong.github.io/account-microscope);
- [Everlastingsong Nebula](https://everlastingsong.github.io/nebula/).
