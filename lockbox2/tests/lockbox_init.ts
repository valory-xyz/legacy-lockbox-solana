import * as idl from "../target/idl/liquidity_lockbox.json";
import * as idl_whirlpool from "../artifacts/whirlpool.json";
import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { LiquidityLockbox } from "../target/types/liquidity_lockbox";
import {
  createMint, mintTo, transfer, getOrCreateAssociatedTokenAccount, syncNative,
  unpackAccount, TOKEN_PROGRAM_ID, AccountLayout, getAssociatedTokenAddress
} from "@solana/spl-token";
import {
  WhirlpoolContext, buildWhirlpoolClient, ORCA_WHIRLPOOL_PROGRAM_ID,
  PDAUtil, PoolUtil, PriceMath, increaseLiquidityQuoteByInputTokenWithParams,
  decreaseLiquidityQuoteByLiquidityWithParams, TickUtil
} from "@orca-so/whirlpools-sdk";
import { DecimalUtil, Percentage } from "@orca-so/common-sdk";
import Decimal from "decimal.js";
import expect from "expect";
import fs from "fs";

// UNIX/Linux/Mac
// bash$ export ANCHOR_PROVIDER_URL=http://127.0.0.1:8899
// bash$ export ANCHOR_WALLET=artifacts/id.json

async function main() {
  // Configure the client to use the local cluster.
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const PROGRAM_ID = new anchor.web3.PublicKey("7ahQGWysExobjeZ91RTsNqTCN3kWyHGZ43ud2vB7VVoZ");
  const program = new Program(idl as anchor.Idl, PROGRAM_ID, anchor.getProvider()) as Program<LiquidityLockbox>;

  const orca = new anchor.web3.PublicKey("whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc");
  const program_whirlpool = new Program(idl_whirlpool as anchor.Idl, orca, anchor.getProvider());

  const whirlpool = new anchor.web3.PublicKey("5dMKUYJDsjZkAD3wiV3ViQkuq9pSmWQ5eAzcQLtDnUT3");
  const sol = new anchor.web3.PublicKey("So11111111111111111111111111111111111111112");
  const olas = new anchor.web3.PublicKey("Ez3nzG9ofodYCvEmw73XhQ87LWNYVRM2s7diB5tBZPyM");
  const tokenVaultA = new anchor.web3.PublicKey("CLA8hU8SkdCZ9cJVLMfZQfcgAsywZ9txBJ6qrRAqthLx");
  const tokenVaultB = new anchor.web3.PublicKey("6E8pzDK8uwpENc49kp5xo5EGydYjtamPSmUKXxum4ybb");
  const tickArrayLower = new anchor.web3.PublicKey("3oJAqTKTCdGvLS9zpoBquWvMjwthu9Np67Qp4W8AT843");
  const tickArrayUpper = new anchor.web3.PublicKey("J3eMJUQWLmSsG5VnXVFHCGwakpKmzi4jkNvi3vbCZQ3o");

  // User wallet is the provider payer
  const userWallet = provider.wallet["payer"];
  console.log("User wallet:", userWallet.publicKey.toBase58());

  const ctx = WhirlpoolContext.withProvider(provider, orca);
  const client = buildWhirlpoolClient(ctx);
  const whirlpoolClient = await client.getPool(whirlpool);

  // Get the current price of the pool
  const sqrt_price_x64 = whirlpoolClient.getData().sqrtPrice;
  const price = PriceMath.sqrtPriceX64ToPrice(sqrt_price_x64, 9, 8);
  console.log("price:", price.toFixed(8));

  // Set price range, amount of tokens to deposit, and acceptable slippage
  const olas_amount = DecimalUtil.toBN(new Decimal("10" /* olas */), 8);
  const sol_amount = DecimalUtil.toBN(new Decimal("10" /* olas */), 9);
  const slippage = Percentage.fromFraction(10, 1000); // 1%
  // Full range price
  const tickSpacing = 64;
  const [lower_tick_index, upper_tick_index] = TickUtil.getFullRangeTickIndex(tickSpacing);


  // Adjust price range (not all prices can be set, only a limited number of prices are available for range specification)
  // (prices corresponding to InitializableTickIndex are available)
  const whirlpool_data = whirlpoolClient.getData();
  const token_a = whirlpoolClient.getTokenAInfo();
  const token_b = whirlpoolClient.getTokenBInfo();

  console.log("lower & upper tick_index:", lower_tick_index, upper_tick_index);
  console.log("lower & upper price:",
    PriceMath.tickIndexToPrice(lower_tick_index, token_a.decimals, token_b.decimals).toFixed(token_b.decimals),
    PriceMath.tickIndexToPrice(upper_tick_index, token_a.decimals, token_b.decimals).toFixed(token_b.decimals)
  );

    // Find a PDA account for the program
    const [pdaProgram, bump] = await anchor.web3.PublicKey.findProgramAddress([Buffer.from("liquidity_lockbox", "utf-8")], program.programId);
    const bumpBytes = Buffer.from(new Uint8Array([bump]));
    console.log("Lockbox PDA address:", pdaProgram.toBase58());
    console.log("Lockbox PDA bump:", bump);

    // Create new bridged token mint with the pda mint authority
    const bridgedTokenMint = await createMint(provider.connection, userWallet, pdaProgram, null, 8);
    console.log("Bridged token mint:", bridgedTokenMint.toBase58());

    let accountInfo = await provider.connection.getAccountInfo(bridgedTokenMint);
    //console.log(accountInfo);

    // Get the tokenA ATA of the program dedicated address for fee collection, and if it does not exist, create it
    const feeCollectorTokenOwnerAccountA = await getOrCreateAssociatedTokenAccount(
        provider.connection,
        userWallet,
        token_a.mint,
        userWallet.publicKey
    );
    console.log("Fee collector ATA for tokenA:", feeCollectorTokenOwnerAccountA.address.toBase58());

    const feeCollectorTokenOwnerAccountB = await getOrCreateAssociatedTokenAccount(
        provider.connection,
        userWallet,
        token_b.mint,
        userWallet.publicKey
    );
    console.log("Fee collector ATA for tokenB:", feeCollectorTokenOwnerAccountB.address.toBase58());

  // Get all teh accounts for the initial zero position
  const positionMintKeypair = anchor.web3.Keypair.generate();
  const positionMint = positionMintKeypair.publicKey;
  console.log("positionMint:", positionMint.toBase58());
  const positionPda = PDAUtil.getPosition(orca, positionMint);
  const position = positionPda.publicKey;
  console.log("position:", position.toBase58());

  // ATA for the PDA to store the position NFT
  const pdaPositionAccount = await getAssociatedTokenAddress(
      positionMint,
      pdaProgram,
      true // allowOwnerOffCurve - allow pda accounts to be have associated token account
  );
  console.log("PDA ATA:", pdaPositionAccount.toBase58());

  let signature = null;

  // Create a liquidity position
    try {
      signature = await program_whirlpool.methods.openPosition(
        positionPda.bump,
        lower_tick_index,
        upper_tick_index)
        .accounts(
          {
            funder: userWallet.publicKey,
            owner: pdaProgram,
            position: position,
            positionMint: positionMint,
            positionTokenAccount: pdaPositionAccount,
            whirlpool
          }
        )
        .signers([positionMintKeypair])
        .rpc();
    } catch (error) {
        if (error instanceof Error && "message" in error) {
            console.error("Program Error:", error);
            console.error("Error Message:", error.message);
        } else {
            console.error("Transaction Error:", error);
        }
    }

    // Initialize the LiquidityLockbox state
    try {
        signature = await program.methods
          .initialize()
          .accounts(
            {
              bridgedTokenMint: bridgedTokenMint,
              feeCollectorTokenOwnerAccountA: feeCollectorTokenOwnerAccountA.address,
              feeCollectorTokenOwnerAccountB: feeCollectorTokenOwnerAccountB.address,
              position: position,
              pdaPositionAccount,
              whirlpool
            }
          )
          .rpc();
    } catch (error) {
        if (error instanceof Error && "message" in error) {
            console.error("Program Error:", error);
            console.error("Error Message:", error.message);
        } else {
            console.error("Transaction Error:", error);
        }
    }
    //console.log("Your transaction signature", signature);
    // Wait for program creation confirmation
    await provider.connection.confirmTransaction({
        signature: signature,
        ...(await provider.connection.getLatestBlockhash()),
    });

    console.log("Successfully initialized lockbox");
}

main();
