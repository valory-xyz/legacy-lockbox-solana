# Solana Sandbox
Set of contracts on Solana. For instructions, please read the README of a corresponding project.

# Current projects
[orca/cpi](https://github.com/valory-xyz/solana-sandbox/tree/main/orca/cpi)
[orca/lockbox](https://github.com/valory-xyz/solana-sandbox/tree/main/orca/lockbox)
[orca/swaps](https://github.com/valory-xyz/solana-sandbox/tree/main/orca/swaps)

## Orca Reference
Instructions can be combined and are usually used in the following combinations
Close also executes Harvest and Withdraw together.
Since some programs may perform Withdraw at the same time as Harvest, the purpose of the entire transaction may not be expressed in a single word.

### Deposit.
openPosition / openPositionWithMetadata
increaseLiquidity

### Deposit to opened position
increaseLiquidity

### Withdraw partial
decreaseLiquidity

### Harvest
updateFeesAndRewards
collectFees
collectReward

### Close (Withdraw all & burn NFT)
updateFeesAndRewards
collectFees
collectReward
decreaseLiquidity
closePosition

small sample of whirlpool's instruction detection
https://github.com/everlastingsong/solsandbox/blob/main/orca/whirlpool/whirlpools_sdk/84a_parse_whirlpool_tx.ts
