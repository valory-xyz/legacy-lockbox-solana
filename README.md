# Lockbox Solana
Set of lockbox contracts on Solana.

# Current implementations
[lockbox](https://github.com/valory-xyz/lockbox-solana/tree/mainlockbox)
[lockbox2](https://github.com/valory-xyz/lockbox-solana/tree/mainlockbox2)

## Orca Reference
Instructions can be combined and are usually used in the following combinations
Close also executes Harvest and Withdraw together.
Since some programs may perform Withdraw at the same time as Harvest, the purpose of the entire transaction may not be expressed in a single word.

### Deposit
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
