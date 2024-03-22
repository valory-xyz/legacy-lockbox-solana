import * as idl from "../target/idl/liquidity_lockbox.json";
import * as idl_whirlpool from "../artifacts/whirlpool.json";
import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { LiquidityLockbox } from "../target/types/liquidity_lockbox";
import {
  createMint, mintTo, transfer, getOrCreateAssociatedTokenAccount, syncNative,
  unpackAccount, TOKEN_PROGRAM_ID, AccountLayout, getAssociatedTokenAddress, setAuthority, AuthorityType
} from "@solana/spl-token";
import {
  WhirlpoolContext, buildWhirlpoolClient, ORCA_WHIRLPOOL_PROGRAM_ID,
  PDAUtil, PoolUtil, PriceMath, decreaseLiquidityQuoteByLiquidityWithParams, TickUtil
} from "@orca-so/whirlpools-sdk";
import { DecimalUtil, Percentage } from "@orca-so/common-sdk";
import Decimal from "decimal.js";
import expect from "expect";
import fs from "fs";

// UNIX/Linux/Mac
// bash$ export ANCHOR_PROVIDER_URL=https://api.devnet.solana.com
// bash$ export ANCHOR_WALLET=id.json

async function main() {
  // Configure the client to use the local cluster.
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  // Program key must be correctly set here
  const PROGRAM_ID = new anchor.web3.PublicKey("1BoXeb8hobfLCHNsyCoG1jpEv41ez4w4eDrJ48N1jY3");
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

  const lockbox = new anchor.web3.PublicKey("3UaaD3puPemoZk7qFYJWWCvmN6diS7P63YR4Si9QRpaW");
  const positionMint = new anchor.web3.PublicKey("36WxSP8trn5czobJaa2Ka7jN58B7sCN7xx2HDom6TDEh");
  const position = new anchor.web3.PublicKey("EHQbFx7m5gPBqXXiViNBfHJDRUuFgqqYsLzuWu18ckaR");
  const pdaPositionAccount = new anchor.web3.PublicKey("sVFBxraUUqmiVFeruh1M7bZS9yuNcoH7Nysh3YTSnZJ");
  const bridgedTokenMint = new anchor.web3.PublicKey("CeZ77ti3nPAmcgRkBkUC1JcoAhR8jRti2DHaCcuyUnzR");
  const feeCollectorTokenOwnerAccountA = new anchor.web3.PublicKey("Gn7oD4PmQth4ehA4b8PpHzq5v1UXPL61jAZd6CSuPvFU");
  const feeCollectorTokenOwnerAccountB = new anchor.web3.PublicKey("FPaBgHbaJR39WBNn6xZRAmurQCBH9QSNWZ5Kk26cGs9d");

  // User wallet is the provider payer
  const userWallet = provider.wallet["payer"];
  console.log("User wallet:", userWallet.publicKey.toBase58());

  const ctx = WhirlpoolContext.withProvider(provider, orca);
  const client = buildWhirlpoolClient(ctx);
  const whirlpoolClient = await client.getPool(whirlpool);

  // Full range price
  const tickSpacing = 64;
  const [lower_tick_index, upper_tick_index] = TickUtil.getFullRangeTickIndex(tickSpacing);

  // Whirlpool tokens
  const whirlpool_data = whirlpoolClient.getData();
  const token_a = whirlpoolClient.getTokenAInfo();
  const token_b = whirlpoolClient.getTokenBInfo();

  // Set price range, amount of tokens to deposit, and acceptable slippage
  const slippage = Percentage.fromFraction(10, 1000); // 1%

  // Get the status of the position
  const positionSDK = await client.getPosition(position);
  const data = positionSDK.getData();

    // Get the ATA of the userWallet address, and if it does not exist, create it
    // This account will have bridged tokens
    const bridgedTokenAccount = await getOrCreateAssociatedTokenAccount(
        provider.connection,
        userWallet,
        bridgedTokenMint,
        userWallet.publicKey
    );
    console.log("User ATA for bridged:", bridgedTokenAccount.address.toBase58());

  // Find bridged token amount
  let tokenAccounts = await provider.connection.getTokenAccountsByOwner(
    userWallet.publicKey,
    { programId: TOKEN_PROGRAM_ID }
  );

  let bridgedTokenAmount = "";
  tokenAccounts.value.forEach((tokenAccount) => {
    const accountData = AccountLayout.decode(tokenAccount.account.data);
    if (accountData.mint.toString() == bridgedTokenMint.toString()) {
      console.log("User ATA bridged balance:", accountData.amount.toString());
      bridgedTokenAmount = accountData.amount.toString();
    }
  });

  let quote = decreaseLiquidityQuoteByLiquidityWithParams({
    // Pass the pool state as is
    sqrtPrice: whirlpool_data.sqrtPrice,
    tickCurrentIndex: whirlpool_data.tickCurrentIndex,
    // Pass the price range of the position as is
    tickLowerIndex: data.tickLowerIndex,
    tickUpperIndex: data.tickUpperIndex,
    // Liquidity to be withdrawn
    liquidity: new anchor.BN(bridgedTokenAmount),
    // Acceptable slippage
    slippageTolerance: slippage,
  });
  console.log("tokenMinA", quote.tokenMinA.toString());
  console.log("tokenMinB", quote.tokenMinB.toString());
  console.log("liquidity", quote.liquidityAmount.toString());

    // Get the tokenA ATA of the userWallet address, and if it does not exist, create it
    const tokenOwnerAccountA = await getOrCreateAssociatedTokenAccount(
        provider.connection,
        userWallet,
        token_a.mint,
        userWallet.publicKey
    );
    console.log("User ATA for tokenA:", tokenOwnerAccountA.address.toBase58());

    // Get the tokenA ATA of the userWallet address, and if it does not exist, create it
    const tokenOwnerAccountB = await getOrCreateAssociatedTokenAccount(
        provider.connection,
        userWallet,
        token_b.mint,
        userWallet.publicKey
    );
    console.log("User ATA for tokenB:", tokenOwnerAccountB.address.toBase58());

    // Execute the correct withdraw tx
    console.log("Amount of bridged tokens to withdraw:", quote.liquidityAmount.toString());
    let signature;
    try {
        signature = await program.methods.withdraw(quote.liquidityAmount, quote.tokenMinA, quote.tokenMinB)
          .accounts(
              {
                lockbox: lockbox,
                whirlpoolProgram: orca,
                whirlpool: whirlpool,
                tokenProgram: TOKEN_PROGRAM_ID,
                position: position,
                positionMint: positionMint,
                bridgedTokenAccount: bridgedTokenAccount.address,
                bridgedTokenMint: bridgedTokenMint,
                pdaPositionAccount: pdaPositionAccount,
                tokenOwnerAccountA: tokenOwnerAccountA.address,
                tokenOwnerAccountB: tokenOwnerAccountB.address,
                feeCollectorTokenOwnerAccountA: feeCollectorTokenOwnerAccountA,
                feeCollectorTokenOwnerAccountB: feeCollectorTokenOwnerAccountB,
                tokenVaultA: tokenVaultA,
                tokenVaultB: tokenVaultB,
                tickArrayLower: tickArrayLower,
                tickArrayUpper: tickArrayUpper
              }
          )
          .signers([userWallet])
          .rpc();
    } catch (error) {
        if (error instanceof Error && "message" in error) {
            console.error("Program Error:", error);
            console.error("Error Message:", error.message);
        } else {
            console.error("Transaction Error:", error);
        }
    }
    console.log("Withdraw tx signature", signature);
    // tx: 5HZG8kLu4Aqbop8xY3QBh8cNEtL1UZSgiuse8fVwpg65u1GcvsXEc3kqttDtvK7dqSrFBJD16qcT5tXzfSmtdEf8

  tokenAccounts = await provider.connection.getTokenAccountsByOwner(
    userWallet.publicKey,
    { programId: TOKEN_PROGRAM_ID }
  );

  tokenAccounts.value.forEach((tokenAccount) => {
    const accountData = AccountLayout.decode(tokenAccount.account.data);
    if (accountData.mint.toString() == bridgedTokenMint.toString()) {
      console.log("User ATA bridged balance now:", accountData.amount.toString());
    }
  });

  console.log("liquidity(after first withdraw):", (await positionSDK.refreshData()).liquidity.toString());

}

main();
