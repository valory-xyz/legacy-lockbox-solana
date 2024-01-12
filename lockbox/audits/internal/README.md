# Internal audit of lockbox-solana
The review has been performed based on the contract code in the following repository:<br>
`https://github.com/valory-xyz/lockbox-solana` <br>
commit: `ae2a7c326124f63c0601a18450972cef43b5ee9f` or `v0.1.0-pre-internal-audit`<br> 

## Objectives
The audit focused on contracts in folder `lockbox`.

### Flatten version
N/A

### OS Requirments checks
Pre-requisites
```
anchor --version
anchor-cli 0.26.0
solana --version
solana-cli 1.14.29 (src:36af529e; feat:139196142)
rustc --version
rustc 1.62.0 (a8314ef7d 2022-06-27)
```
Checks - passed [x]
```
audit/script/setup-env-old.sh
anchor --version
anchor-cli 0.26.0
solana --version
solana-cli 1.14.29 (src:36af529e; feat:139196142)
rustc --version
rustc 1.62.0 (a8314ef7d 2022-06-27)
```


## Security issues.
### Problems found instrumentally
Several checks are obtained automatically. They are commented. Some issues found need to be fixed. <br>
Warning: Due to the rust specific, you need to upgrade evn to use these tools and do a downgrade before `anchor build` 
```
audits/script/setup-env-latest.sh
cargo-audit audit
...
audits/script/setup-env-old.sh 
```
List of rust tools:
##### cargo tree
```
cargo tree > audits/internal/analysis/cargo_tree.txt
```
##### cargo-audit
https://docs.rs/cargo-audit/latest/cargo_audit/
```
cargo install cargo-audit
cargo-audit audit > audits/internal/analysis/cargo-audit.txt
```
[x] Out of scope

##### cargo clippy 
https://github.com/rust-lang/rust-clippy
```
cargo clippy 2> audits/internal/analysis/cargo-clippy.txt
```
[x] re-run the script as the structs have changed

##### cargo-geiger
https://doc.rust-lang.org/book/ch19-01-unsafe-rust.html
https://github.com/geiger-rs/cargo-geiger?tab=readme-ov-file
```
cargo install --locked cargo-geiger
cd lockbox/programs/liquidity_lockbox
cargo-geiger > audits/internal/analysis/cargo-geiger.txt
```
[x] "!" is out of scope

##### cargo-spellcheck
https://github.com/drahnr/cargo-spellcheck
```
sudo apt install llvm
llvm-config --prefix 
/usr/lib/llvm-14
sudo apt-get install libclang-dev
cargo install --locked cargo-spellcheck
cd programs/liquidity_lockbox/
cargo spellcheck -r list-files
/home/andrey/valory/lockbox-solana/lockbox/programs/liquidity_lockbox/src/lib.rs
/home/andrey/valory/lockbox-solana/lockbox/programs/liquidity_lockbox/src/state.rs
cargo spellcheck --verbose check
```
All automatic warnings are listed in the following file, concerns of which we address in more detail below: <br>
[cargo-tree.txt](https://github.com/valory-xyz/lockbox-solana//blob/main/lockbox/audits/internal/analysis/cargo-tree.txt) <br>
[cargo-audit.txt](https://github.com/valory-xyz/lockbox-solana//blob/main/lockbox/audits/internal/analysis/cargo-audit.txt) <br>
[cargo-clippy.txt](https://github.com/valory-xyz/lockbox-solana//blob/main/lockbox/audits/internal/analysis/cargo-clippy.txt) <br>
[cargo-geiger.txt](https://github.com/valory-xyz/lockbox-solana//blob/main/lockbox/audits/internal/analysis/cargo-geiger.txt) <br>
Notes: <br>
https://rustsec.org/advisories/RUSTSEC-2022-0093 - out of scope

Pay attention: <br>
Tools for fuzzing: <br>
https://ackeeblockchain.com/blog/introducing-trdelnik-fuzz-testing-framework-for-solana-and-anchor/


### Problems found by manual analysis 05.01.23

List of attack vectors <br>
https://www.sec3.dev/blog/how-to-audit-solana-smart-contracts-part-1-a-systematic-approach <br>
https://medium.com/@zokyo.io/what-hackers-look-for-in-a-solana-smart-contract-17ec02b69fb6 <br>
1. Missing signer checks (e.g., by checking AccountInfo::is_signer ) <br>
N/A

2. Missing ownership checks (e.g., by checking  AccountInfo::owner) <br>
Example: https://github.com/coral-xyz/sealevel-attacks/blob/master/programs/1-account-data-matching/recommended/src/lib.rs <br>
Please, double checking ownership
In deposit:
pda_position_account.owner == lockbox.key() [x]
position_token_account.owner ?
bridged_token_mint.owner ?
bridged_token_account.owner ?
lockbox.owner ?

In withdraw:
bridged_token_mint.owner ?
bridged_token_account.owner ?
position.owner ?
pda_position_account.owner ?
position_mint.owner ?
token_owner_account_a.owner ?
token_owner_account_b.owner ?
token_vault_a.owner ?
token_vault_b.owner ?
tick_array_lower.owner ?
tick_array_upper.onwer ?
To discussion.
[x] Fixed.

3. Missing rent exemption checks <br>
? In progress
- approach (lockbox.position_accounts.push) limited 10k/(2 * sizeof(pubkey) + sizeof(u64)) ~ 138 NFT. So, after it the account will be filled
we need some other solution.
Notes: PDA account can't be > 10k. ref: https://stackoverflow.com/questions/70150946/systemprogramcreateaccount-data-size-limited-to-10240-in-inner-instructions
To discussion.
Status at the time of audit: will be corrected in the next version.
[x] Resolved all big accounts. Need to re-audit. Need to build an off-chain system that monitors sensitive position related accounts.

4. Signed invocation of unverified programs <br>
token_program is real token_program ?
whirlpool_program is real whirlpool_program ?
To discussion.
[x] Fixed.


5. Solana account confusions: the program fails to ensure that the account data has the type it expects to have. <br>
lockbox.bridged_token_mint vs account.bridged_token_mint ? // Check that the bridged token mint account is correct
pub whirlpool: Box<Account<'info, Whirlpool>> is whirlpool ?
To discussion.

6. Re-initiation with cross-instance confusion <br>
Passed. Example: https://github.com/coral-xyz/sealevel-attacks/blob/master/programs/4-initialization/recommended/src/lib.rs
[x] In place.

7. Arithmetic overflow/underflows: If an arithmetic operation results in a higher or lower value, the value will wrap around with two’s complement. <br>
Failed. Pay attention.
```
https://stackoverflow.com/questions/52646755/checking-for-integer-overflow-in-rust
https://doc.rust-lang.org/std/primitive.u32.html#method.checked_add
```
[x] Fixed.

8. Numerical precision errors: numeric calculations on floating point can cause precision errors and those errors can accumulate. <br>
N/A

9. Loss of precision in calculation: numeric calculations on integer types such as division can loss precision. <br>
N/A

10. Incorrect calculation: for example, incorrect numerical computes due to copy/paste errors <br>
Passed.

11. Casting truncation <br>
N/A

12. Exponential complexity in calculation <br>
Passed.

13. Missing freeze authority checks <br>
[x] Checked, all position authority is null.

14. Insufficient SPL-Token account verification <br>
bridged_token is SPL-token ?
[x] Checked, bridge token mint is the protocol token mint.

#### General notes not specific to Solana/Rust. Critical
##### No event in `deposit`
##### No event in `withdraw`
[x] Fixed.

### Notes:
####  Rare case with try_find_program_address => None
```
    ref: https://docs.rs/solana-program/latest/solana_program/pubkey/struct.Pubkey.html#method.try_find_program_address
    let position_pda = Pubkey::try_find_program_address(&[b"position", position_mint.as_ref()], &ORCA);
    position_pda is None?
    let position_pda_pubkey = position_pda.map(|(pubkey, _)| pubkey);

    maybe https://docs.rs/solana-program/latest/solana_program/pubkey/struct.Pubkey.html#method.find_program_address

```
[x] Fixed.

#### Documentation standard in Rust
Discussion with examples: <br>
https://community.starknet.io/t/revisiting-the-comment-standard-natspec-or-rust/98009/6 <br>
https://doc.rust-lang.org/rust-by-example/meta/doc.html <br>
[x] Discussed

#### Negative tests. 
1. re-initialize await program.methods.initialize()
2. program.methods.deposit() with wrong/fake accounts
3. program.methods.withdraw()  with wrong/fake accounts
[x] Fixed

#### Fixed and removed all TODO
[x] Fixed

#### Clean tests (delete commented/unused code)
[x] Fixed

#### Cleaning repo
Is it possible to make a project containing only `lockbox`
[x] Fixed

## Re-audit 09.01.24
The review has been performed based on the contract code in the following repository:<br>
`https://github.com/valory-xyz/lockbox-solana` <br>
commit: `64ebb0f0129dde2de1226931f22aaeb218885ee8` or `v0.1.1-pre-internal-audit`<br> 

## Security issues.
### Problems found instrumentally
##### cargo clippy 
https://github.com/rust-lang/rust-clippy
```
cargo clippy 2> audits/internal/analysis/cargo-clippy-2.txt
```
re-run. Pay attention to result of run. 
[cargo-clippy-2.txt](https://github.com/valory-xyz/lockbox-solana//blob/main/lockbox/audits/internal/analysis/cargo-clippy-2.txt) <br>
[x] Fixed.

##### Sec3 x-ray scanner
```
These two accounts are both mutable and may be the same account
lockbox/programs/liquidity_lockbox/src/lib.rs:558
  )]
  pub token_vault_a: Box<Account<'info, TokenAccount>>,
  #[account(mut, constraint = token_vault_b.key() == whirlpool.token_vault_b)]
  pub token_vault_b: Box<Account<'info, TokenAccount>>,
  #[account(mut, has_one = whirlpool)]
  pub tick_array_lower: AccountLoader<'info, TickArray>,
  #[account(mut, has_one = whirlpool)]
  pub tick_array_upper: AccountLoader<'info, TickArray>,
  #[account(mut)]
  pub lockbox: Box<Account<'info, LiquidityLockbox>>,
  pub whirlpool_program: Program<'info, whirlpool::program::Whirlpool>,
lockbox/programs/liquidity_lockbox/src/lib.rs:560

  #[account(mut, constraint = token_vault_b.key() == whirlpool.token_vault_b)]
  pub token_vault_b: Box<Account<'info, TokenAccount>>,
  #[account(mut, has_one = whirlpool)]
  pub tick_array_lower: AccountLoader<'info, TickArray>,
  #[account(mut, has_one = whirlpool)]
  pub tick_array_upper: AccountLoader<'info, TickArray>,
  #[account(mut)]
  pub lockbox: Box<Account<'info, LiquidityLockbox>>,
  pub whirlpool_program: Program<'info, whirlpool::program::Whirlpool>,
  #[account(address = token::ID)]

https://github.com/coral-xyz/sealevel-attacks/tree/master/programs/6-duplicate-mutable-accounts
```
[sec3-report.PNG](https://github.com/valory-xyz/lockbox-solana//blob/main/lockbox/audits/internal/analysis/sec3-report.PNG) <br>
[x] Fixed, see [here](https://github.com/valory-xyz/lockbox-solana//blob/main/lockbox/audits/internal/analysis/sec3-report_fixed.PNG)

##### Missing ownership checks (e.g., by checking  AccountInfo::owner)
[x] Fixed.

## Re-audit 12.01.24
The review has been performed based on the contract code in the following repository:<br>
`https://github.com/valory-xyz/lockbox-solana` <br>
commit: `44bab206d095739a1f4b49ca6fccbe3ca277066d` or `v0.1.2-pre-internal-audit`<br> 

## Security issues.
### Problems found instrumentally
##### cargo clippy 
https://github.com/rust-lang/rust-clippy
```
cargo clippy 2> audits/internal/analysis/cargo-clippy-3.txt
```
re-run.
[cargo-clippy-3.txt](https://github.com/valory-xyz/lockbox-solana//blob/main/lockbox/audits/internal/analysis/cargo-clippy-3.txt) <br>
- no new vulnerabilities were introduced since the previous fix

##### Sec3 x-ray scanner
- No issue, see [here](https://github.com/valory-xyz/lockbox-solana//blob/main/lockbox/audits/internal/analysis/sec3-report-12-01-24.PNG)

