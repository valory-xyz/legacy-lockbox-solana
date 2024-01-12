# Lockbox Solana
Set of lockbox contracts on Solana.

# Current implementations
1. [lockbox](https://github.com/valory-xyz/lockbox-solana/tree/main/lockbox) 
	
	This folder contains the liquidity lockbox v1 set of contracts on Solana. 

	Developments steps, tests, documentation, audits can be found in [this](https://github.com/valory-xyz/lockbox-solana/tree/main/lockbox) folder. 

2. [lockbox2](https://github.com/valory-xyz/lockbox-solana/tree/main/lockbox2)
	
	This folder contains the liquidity lockbox v2 set of contracts on Solana.

	Developments steps, tests, documentation, and audits can be found in [this](https://github.com/valory-xyz/lockbox-solana/tree/main/lockbox2) folder. 


## Pre-requisites
A successful program CPI with Orca Whirlpool program requires that the following environment is satisfied:

```
anchor --version
anchor-cli 0.26.0
solana --version
solana-cli 1.14.29 (src:36af529e; feat:139196142)
rustc --version
rustc 1.62.0 (a8314ef7d 2022-06-27)
```

Advise the script `setup-env.sh` to correctly install the required environment.


## Acknowledgements
The liquidity lockbox contracts were inspired and based on the following sources:
- [Orca](https://github.com/orca-so/whirlpools);
- [EverlastingsongSolsandbox](https://github.com/everlastingsong/solsandbox);
- [Everlastingsong Microscope](https://everlastingsong.github.io/account-microscope);
- [Everlastingsong Nebula](https://everlastingsong.github.io/nebula/).
