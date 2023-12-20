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

Build the code with:
```
anchor build
```

Run the validator in a separate window:
```
./validator.sh
```

Then, execute the testing script:
```
solana airdrop 10000 9fit3w7t6FHATDaZWotpWqN7NpqgL3Lm1hqUop4hAy8h --url localhost && npx ts-node tests/cpi.ts
```

If the `@programId` in lib.rs does not match with the deployed one, update it and re-run
```
anchor build
```

## Acknowledgements
The liquidity lockbox contracts were inspired and based on the following sources:
- [Orca](https://github.com/orca-so/whirlpools);
- [EverlastingsongSolsandbox](https://github.com/everlastingsong/solsandbox);
- [Everlastingsong Microscope](https://everlastingsong.github.io/account-microscope);
- [Everlastingsong Nebula](https://everlastingsong.github.io/nebula/).