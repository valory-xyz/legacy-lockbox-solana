import * as idl from "../target/idl/cpi_whirlpool_increase_liquidity.json";
import type { CpiWhirlpool } from "../target/types/cpi_whirlpool_increase_liquidity";
import * as anchor from "@project-serum/anchor";
import { Program } from "@project-serum/anchor";
import { TOKEN_PROGRAM_ID, ASSOCIATED_TOKEN_PROGRAM_ID, Token } from "@solana/spl-token";
import { PublicKey } from "@solana/web3.js";
import { ORCA_WHIRLPOOL_PROGRAM_ID, WhirlpoolContext, AccountFetcher, buildWhirlpoolClient, PDAUtil } from "@orca-so/whirlpools-sdk";
import { deriveATA } from "@orca-so/common-sdk";

// UNIX/Linux/Mac
// bash$ export ANCHOR_PROVIDER_URL=https://api.devnet.solana.com
// bash$ export ANCHOR_WALLET=~/.config/solana/id.json
// bash$ export WHIRLPOOL_POSITION=<address of position>
// bash$ ts-node this_script.ts

async function main() {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);
  const ctx = WhirlpoolContext.from(anchor.getProvider().connection, anchor.AnchorProvider.env().wallet, ORCA_WHIRLPOOL_PROGRAM_ID);
  const fetcher = new AccountFetcher(ctx.connection);
  const client = buildWhirlpoolClient(ctx, fetcher);

  const whirlpoolProgram = new anchor.web3.PublicKey("whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc");

  const PROGRAM_ID = new anchor.web3.PublicKey("HB95NrGYyYK45UsNy2u4S1cnyJPVrZuCwBggVJwUttuf");
  const program = new Program(idl as anchor.Idl, PROGRAM_ID, anchor.getProvider()) as Program<CpiWhirlpool>;

//  console.log("Provider wallet:", provider.wallet.payer.publicKey.toBase58());
//  const program = anchor.workspace.CpiWhirlpool as Program<CpiWhirlpool>;
//  return;

  // from env
  const position_address = process.env.WHIRLPOOL_POSITION;
  const position_pubkey = new PublicKey(position_address);
  console.log("position address:", position_pubkey.toBase58());

  // get position & pool
  const position = await client.getPosition(position_pubkey);
  const position_owner = ctx.wallet.publicKey;
  const position_token_account = await deriveATA(position_owner, position.getData().positionMint);
  const whirlpool_pubkey = position.getData().whirlpool;
  const whirlpool = await client.getPool(whirlpool_pubkey);
  const token_a = whirlpool.getTokenAInfo();
  const token_b = whirlpool.getTokenBInfo();
  const token_owner_a = await deriveATA(position_owner, token_a.mint);
  const token_owner_b = await deriveATA(position_owner, token_b.mint);
  const token_vault_a = whirlpool.getData().tokenVaultA;
  const token_vault_b = whirlpool.getData().tokenVaultB;

  // get tickarray
  const tick_spacing = whirlpool.getData().tickSpacing;
  const tick_array_lower_pubkey = PDAUtil.getTickArrayFromTickIndex(position.getData().tickLowerIndex, tick_spacing, whirlpool_pubkey, ctx.program.programId).publicKey;
  const tick_array_upper_pubkey = PDAUtil.getTickArrayFromTickIndex(position.getData().tickUpperIndex, tick_spacing, whirlpool_pubkey, ctx.program.programId).publicKey;

  const liquidity = new anchor.BN(11464943);
  const minA = new anchor.BN(0);
  const minB = new anchor.BN(0);

//  console.log(program.programId);
//  return;
  try {
      const signature = await program.rpc.decreaseLiquidity(liquidity, minA, minB, {
        accounts: {
          whirlpool: whirlpool_pubkey,
          position: position_pubkey,
          positionAuthority: position_owner,
          positionTokenAccount: position_token_account,
          tickArrayLower: tick_array_lower_pubkey,
          tickArrayUpper: tick_array_upper_pubkey,
          tokenOwnerAccountA: token_owner_a,
          tokenVaultA: token_vault_a,
          tokenOwnerAccountB: token_owner_b,
          tokenVaultB: token_vault_b,
          whirlpoolProgram: whirlpoolProgram,
          tokenProgram: TOKEN_PROGRAM_ID,
        },
        signers: [],
      });
      console.log("decreaseLiquidity signature", signature);
  } catch (error) {
      if (error instanceof Error && "message" in error) {
          console.error("Program Error:", error);
          console.error("Error Message:", error.message);
      } else {
          console.error("Transaction Error:", error);
      }
  }
}

main();

/*

$ ts-node test.ts 
position address: ACVvYGzmL3k74WHRR4zfmCk6cRt29WoChD4CoGJyiuT3
increaseLiquidity signature 3EGsvrYAaX3K9LdH7vyf7hiQraodStKSL7WU5c7FHuhCaQjb6e9A4vYrfYpjHxKzrWSnHg3pDZSjPzNAr8c5rzuo

*/