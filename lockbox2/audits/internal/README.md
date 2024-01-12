# Internal audit of lockbox-solana
The review has been performed based on the contract code in the following repository:<br>
`https://github.com/valory-xyz/lockbox-solana` <br>
commit: `44bab206d095739a1f4b49ca6fccbe3ca277066d` or `v0.1.2-pre-internal-audit`<br> 

## Objectives
The audit focused on contracts in folder `lockbox2`.

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
[x] Known: Out of scope

##### cargo clippy 
https://github.com/rust-lang/rust-clippy
```
cargo clippy 2> audits/internal/analysis/cargo-clippy.txt
```
[x] Fixed.

##### cargo-geiger
https://doc.rust-lang.org/book/ch19-01-unsafe-rust.html
https://github.com/geiger-rs/cargo-geiger?tab=readme-ov-file
```
cargo install --locked cargo-geiger
cd lockbox2/programs/liquidity_lockbox
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
[cargo-tree.txt](https://github.com/valory-xyz/lockbox-solana/blob/main/lockbox/audits/internal/analysis/cargo-tree.txt) <br>
[cargo-audit.txt](https://github.com/valory-xyz/lockbox-solana/blob/main/lockbox/audits/internal/analysis/cargo-audit.txt) <br>
[cargo-clippy.txt](https://github.com/valory-xyz/lockbox-solana/blob/main/lockbox/audits/internal/analysis/cargo-clippy.txt) <br>
[cargo-geiger.txt](https://github.com/valory-xyz/lockbox-solana/blob/main/lockbox/audits/internal/analysis/cargo-geiger.txt) <br>
Notes: <br>
https://rustsec.org/advisories/RUSTSEC-2022-0093 - out of scope



### Problems found by analysis
##### Sec3 x-ray scanner
- No issue, see [here](https://github.com/valory-xyz/lockbox-solana/blob/main/lockbox2/audits/internal/analysis/sec3-report-12-01-24-lockbox2.PNG)


