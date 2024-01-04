# Internal audit of autonolas-tokenomics
The review has been performed based on the contract code in the following repository:<br>
`https://github.com/valory-xyz/solana-sandbox` <br>
commit: `ae2a7c326124f63c0601a18450972cef43b5ee9f` or `v0.1.0-pre-internal-audit`<br> 

## Objectives
The audit focused on contracts in folder `orca/lockbox`.

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
##### cargo clippy 
https://github.com/rust-lang/rust-clippy
```
cargo clippy 2> audits/internal/analysis/cargo-clippy.txt
```
##### cargo-geiger
https://doc.rust-lang.org/book/ch19-01-unsafe-rust.html
https://github.com/geiger-rs/cargo-geiger?tab=readme-ov-file
```
cargo install --locked cargo-geiger
cd lockbox/programs/liquidity_lockbox
cargo-geiger > audits/internal/analysis/cargo-geiger.txt
```
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
/home/andrey/valory/solana-sandbox/orca/lockbox/programs/liquidity_lockbox/src/lib.rs
/home/andrey/valory/solana-sandbox/orca/lockbox/programs/liquidity_lockbox/src/state.rs
cargo spellcheck --verbose check
```
All automatic warnings are listed in the following file, concerns of which we address in more detail below: <br>
[cargo-tree.txt](https://github.com/valory-xyz/solana-sandbox//blob/main/orca/lockbox/audits/internal/analysis/cargo-tree.txt) <br>
[cargo-audit.txt](https://github.com/valory-xyz/solana-sandbox//blob/main/orca/lockbox/audits/internal/analysis/cargo-audit.txt) <br>
[cargo-clippy.txt](https://github.com/valory-xyz/solana-sandbox//blob/main/orca/lockbox/audits/internal/analysis/cargo-clippy.txt) <br>
[cargo-geiger.txt](https://github.com/valory-xyz/solana-sandbox//blob/main/orca/lockbox/audits/internal/analysis/cargo-geiger.txt) <br>
Notes: <br>
https://rustsec.org/advisories/RUSTSEC-2022-0093 - out of scope

Pay attention: <br>
Tools for fuzzing: <br>
https://ackeeblockchain.com/blog/introducing-trdelnik-fuzz-testing-framework-for-solana-and-anchor/


### Problems found by manual analysis 04.01.23

List of attack vectors <br>
https://www.sec3.dev/blog/how-to-audit-solana-smart-contracts-part-1-a-systematic-approach <br>
https://medium.com/@zokyo.io/what-hackers-look-for-in-a-solana-smart-contract-17ec02b69fb6 <br>
1. Missing signer checks (e.g., by checking AccountInfo::is_signer ) <br>
N/A

2. Missing ownership checks (e.g., by checking  AccountInfo::owner) <br>
Example: https://github.com/coral-xyz/sealevel-attacks/blob/master/programs/1-account-data-matching/recommended/src/lib.rs <br>
In progress.

3. Missing rent exemption checks <br>
? In progress

4. Signed invocation of unverified programs <br>
In progress. To discussion.

5. Solana account confusions: the program fails to ensure that the account data has the type it expects to have. <br>
In progress.

6. Re-initiation with cross-instance confusion <br>
Passed. Example: https://github.com/coral-xyz/sealevel-attacks/blob/master/programs/4-initialization/recommended/src/lib.rs

7. Arithmetic overflow/underflows: If an arithmetic operation results in a higher or lower value, the value will wrap around with two’s complement. <br>
Failed. Pay attention.
```
Most likely low level issue.
https://stackoverflow.com/questions/52646755/checking-for-integer-overflow-in-rust
https://doc.rust-lang.org/std/primitive.u32.html#method.checked_add
```
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
? In progress

14. Insufficient SPL-Token account verification <br>
? In progress

#### General notes not specific to Solana/Rust. Critical
##### No event in `deposit`
##### No event in `withdraw`

### Notes:
####  Rare case with try_find_program_address => None
```
    ref: https://docs.rs/solana-program/latest/solana_program/pubkey/struct.Pubkey.html#method.try_find_program_address
    let position_pda = Pubkey::try_find_program_address(&[b"position", position_mint.as_ref()], &ORCA);
    position_pda is None?
    let position_pda_pubkey = position_pda.map(|(pubkey, _)| pubkey);

```

#### Documentation standard in Rust
Discussion with examples: <br>
https://community.starknet.io/t/revisiting-the-comment-standard-natspec-or-rust/98009/6 <br>
https://doc.rust-lang.org/rust-by-example/meta/doc.html <br>

#### Negative tests. 
1. re-initialize await program.methods.initialize()
2. program.methods.deposit() with wrong/fake accounts
3. program.methods.withdraw()  with wrong/fake accounts

#### Fixed and removed all TODO

#### Clean tests (delete commented/unused code)

#### Cleaning repo
Is it possible to make a project containing only `lockbox`