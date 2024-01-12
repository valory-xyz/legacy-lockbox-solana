# Lockbox Solana
Set of lockbox contracts on Solana.

# Current implementations
1. [lockbox](https://github.com/valory-xyz/lockbox-solana/tree/main/lockbox) 
	
	This folder contains the liquidity lockbox v1 set of contracts on Solana. 

	Developments steps, tests, documentation, audits can be found in [this]https://github.com/valory-xyz/lockbox-solana/tree/main/lockbox) folder. 

2. [lockbox2](https://github.com/valory-xyz/lockbox-solana/tree/main/lockbox2)
	
	This folder contains the liquidity lockbox v2 set of contracts on Solana.

	Developments steps, tests, documentation, and audits can be found in [this](https://github.com/valory-xyz/lockbox-solana/tree/main/lockbox2) folder. 


## Pre-requisites

```
anchor --version
anchor-cli 0.26.0
solana --version
solana-cli 1.14.29 (src:36af529e; feat:139196142)
rustc --version
rustc 1.62.0 (a8314ef7d 2022-06-27)
```

### Instruction for first installations of pre-requisites

Run the script setup-env.sh with following commard and follows the script instructions

```
./setup-env.sh 
```

At the end the script, the following have been installed

```
solana --version
solana-cli 1.14.29 (src:36af529e; feat:139196142)
cargo --version
cargo 1.75.0 (1d8b05cdd 2023-11-20)
rustc --version
rustc 1.62.0 (a8314ef7d 2022-06-27)
rustc 1.75.0 (82e1608df 2023-12-21)
```

Select the correct versions by running the following commands

```
rustup install 1.62
cargo install --git https://github.com/coral-xyz/anchor avm --locked --force
avm install 0.26.0
rustup default 1.62
avm use 0.26.0
```


## Acknowledgements
The liquidity lockbox contracts were inspired and based on the following sources:
- [Orca](https://github.com/orca-so/whirlpools);
- [EverlastingsongSolsandbox](https://github.com/everlastingsong/solsandbox);
- [Everlastingsong Microscope](https://everlastingsong.github.io/account-microscope);
- [Everlastingsong Nebula](https://everlastingsong.github.io/nebula/).
