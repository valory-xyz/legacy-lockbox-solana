import * as idl from "../target/idl/cpi_whirlpool.json";
import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import type { CpiWhirlpool } from "../target/types/cpi_whirlpool";
import { createMint, mintTo, transfer, getOrCreateAssociatedTokenAccount, unpackAccount, TOKEN_PROGRAM_ID } from "@solana/spl-token";
import {
  WhirlpoolContext, buildWhirlpoolClient, ORCA_WHIRLPOOL_PROGRAM_ID,
  PDAUtil, PoolUtil, PriceMath, increaseLiquidityQuoteByInputTokenWithParams,
  decreaseLiquidityQuoteByLiquidityWithParams
} from "@orca-so/whirlpools-sdk";
import { DecimalUtil, Percentage } from "@orca-so/common-sdk";
import Decimal from "decimal.js";
import expect from "expect";

// UNIX/Linux/Mac
// bash$ export ANCHOR_PROVIDER_URL=http://127.0.0.1:8899
// bash$ export ANCHOR_WALLET=artifacts/id.json

async function main() {
  // Configure the client to use the local cluster.
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const PROGRAM_ID = new anchor.web3.PublicKey("HB95NrGYyYK45UsNy2u4S1cnyJPVrZuCwBggVJwUttuf");
  const program = new Program(idl as anchor.Idl, PROGRAM_ID, anchor.getProvider()) as Program<CpiWhirlpool>;

  const orca = new anchor.web3.PublicKey("whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc");
  const whirlpool = new anchor.web3.PublicKey("7qbRF6YsyGuLUVs6Y1q64bdVrfe4ZcUUz1JRdoVNUJnm");
  const sol = new anchor.web3.PublicKey("So11111111111111111111111111111111111111112");
  const usdc = new anchor.web3.PublicKey("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v");
  const tokenVaultA = new anchor.web3.PublicKey("9RfZwn2Prux6QesG1Noo4HzMEBv3rPndJ2bN2Wwd6a7p");
  const tokenVaultB = new anchor.web3.PublicKey("BVNo8ftg2LkkssnWT4ZWdtoFaevnfD6ExYeramwM27pe");
  const tickArrayLower = new anchor.web3.PublicKey("DJBLVHo3uTQBYpSHbVdDq8LoRsSiYV9EVhDUguXszvCi");
  const tickArrayUpper = new anchor.web3.PublicKey("ZPyVkTuj9TBr1ER4Fnubyz1w7bm5LsXctLiZb8Fs2Do");

    // User wallet is the provider payer
    const userWallet = provider.wallet["payer"];
    console.log("User wallet:", userWallet.publicKey.toBase58());

      const ctx = WhirlpoolContext.withProvider(provider, orca);
      const client = buildWhirlpoolClient(ctx);
      const whirlpoolClient = await client.getPool(whirlpool);

      // Get the current price of the pool
      const sqrt_price_x64 = whirlpoolClient.getData().sqrtPrice;
      const price = PriceMath.sqrtPriceX64ToPrice(sqrt_price_x64, 9, 6);
      console.log("price:", price.toFixed(6));

      // Set price range, amount of tokens to deposit, and acceptable slippage
      const usdc_amount = DecimalUtil.toBN(new Decimal("10" /* usdc */), 6);
      const slippage = Percentage.fromFraction(10, 1000); // 1%
      // Full range price
      const lower_tick_index = -443632;
      const upper_tick_index = 443632;

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

      // Obtain deposit estimation
      let quote = increaseLiquidityQuoteByInputTokenWithParams({
        // Pass the pool definition and state
        tokenMintA: token_a.mint,
        tokenMintB: token_b.mint,
        sqrtPrice: whirlpool_data.sqrtPrice,
        tickCurrentIndex: whirlpool_data.tickCurrentIndex,
        // Price range
        tickLowerIndex: lower_tick_index,
        tickUpperIndex: upper_tick_index,
        // Input token and amount
        inputTokenMint: usdc,
        inputTokenAmount: usdc_amount,
        // Acceptable slippage
        slippageTolerance: slippage,
      });

      // Output the estimation
      console.log("SOL max input:", DecimalUtil.fromBN(quote.tokenMaxA, token_a.decimals).toFixed(token_a.decimals));
      console.log("USDC max input:", DecimalUtil.fromBN(quote.tokenMaxB, token_b.decimals).toFixed(token_b.decimals));

      // Create a transaction
      // Use openPosition method instead of openPositionWithMetadata method
      const open_position_tx = await whirlpoolClient.openPosition(
        lower_tick_index,
        upper_tick_index,
        quote
      );

      // Send the transaction to open a position
      let signature = await open_position_tx.tx.buildAndExecute();
      console.log("signature:", signature);
      console.log("position NFT:", open_position_tx.positionMint.toBase58());
      const positionMint = open_position_tx.positionMint;

      // Wait for the transaction to complete
      let latest_blockhash = await ctx.connection.getLatestBlockhash();
      await ctx.connection.confirmTransaction({signature, ...latest_blockhash}, "confirmed");

    // Get all token accounts
    const token_accounts = (await ctx.connection.getTokenAccountsByOwner(ctx.wallet.publicKey, {programId: TOKEN_PROGRAM_ID})).value;

    let parsed;
    let position;
    for (let i = 0; i < token_accounts.length; i++) {
        const ta = token_accounts[i];
        parsed = unpackAccount(ta.pubkey, ta.account);
        if (parsed.amount.toString() === "1") {
            position = PDAUtil.getPosition(ctx.program.programId, parsed.mint);
            break;
        }
    }

    // NFT position mint
    let accountInfo = await provider.connection.getAccountInfo(positionMint);
    //console.log(accountInfo);

    // Get the ATA of the userWallet address, and if it does not exist, create it
    // This account has an NFT token
    const userPositionAccount = parsed.address;
    console.log("User ATA for NFT:", userPositionAccount.toBase58());

    // Get the tokenA ATA of the userWallet address, and if it does not exist, create it
    // This account will have bridged tokens
    const userTokenAccountA = await getOrCreateAssociatedTokenAccount(
        provider.connection,
        userWallet,
        token_a.mint,
        userWallet.publicKey
    );
    console.log("User ATA for tokenA:", userTokenAccountA.address.toBase58());

    // Get the tokenA ATA of the userWallet address, and if it does not exist, create it
    // This account will have bridged tokens
    const userTokenAccountB = await getOrCreateAssociatedTokenAccount(
        provider.connection,
        userWallet,
        token_b.mint,
        userWallet.publicKey
    );
    console.log("User ATA for tokenB:", userTokenAccountB.address.toBase58());

    // Get the status of the position
    const positionSDK = await client.getPosition(position.publicKey);
    const data = positionSDK.getData();

    // Get the price range of the position
    const lower_price = PriceMath.tickIndexToPrice(data.tickLowerIndex, token_a.decimals, token_b.decimals);
    const upper_price = PriceMath.tickIndexToPrice(data.tickUpperIndex, token_a.decimals, token_b.decimals);

    // Calculate the amount of tokens that can be withdrawn from the position
    const amounts = PoolUtil.getTokenAmountsFromLiquidity(
      data.liquidity,
      whirlpoolClient.getData().sqrtPrice,
      PriceMath.tickIndexToSqrtPriceX64(data.tickLowerIndex),
      PriceMath.tickIndexToSqrtPriceX64(data.tickUpperIndex),
      true
    );

    // Output the status of the position
    console.log("position:", position.publicKey.toBase58());
    console.log("\twhirlpool address:", data.whirlpool.toBase58());
    console.log("\ttokenA:", token_a.mint.toBase58());
    console.log("\ttokenB:", token_b.mint.toBase58());
    console.log("\tliquidity:", data.liquidity.toNumber());
    console.log("\tlower:", data.tickLowerIndex, lower_price.toFixed(token_b.decimals));
    console.log("\tupper:", data.tickUpperIndex, upper_price.toFixed(token_b.decimals));
    console.log("\tamountA:", DecimalUtil.fromBN(amounts.tokenA, token_a.decimals).toString());
    console.log("\tamountB:", DecimalUtil.fromBN(amounts.tokenB, token_b.decimals).toString());

//  // Test decrease liquidity with the SDK
//  // Set the percentage of liquidity to be withdrawn (30%)
//  const delta_liquidity = data.liquidity.mul(new anchor.BN(30)).div(new anchor.BN(100));
//  console.log(delta_liquidity.toNumber());
//
//  quote = decreaseLiquidityQuoteByLiquidityWithParams({
//    // Pass the pool state as is
//    sqrtPrice: whirlpool_data.sqrtPrice,
//    tickCurrentIndex: whirlpool_data.tickCurrentIndex,
//    // Pass the price range of the position as is
//    tickLowerIndex: data.tickLowerIndex,
//    tickUpperIndex: data.tickUpperIndex,
//    // Liquidity to be withdrawn
//    liquidity: delta_liquidity,
//    // Acceptable slippage
//    slippageTolerance: slippage,
//  });
//  console.log("quote", quote);
//
//  // Create a transaction
//  const decrease_liquidity_tx = await positionSDK.decreaseLiquidity(quote);
//  // Overwrite the tokenA ATA as it is the only difference
//  decrease_liquidity_tx.instructions[2].instructions[0].keys[5].pubkey = userTokenAccountA.address;
//  console.log(decrease_liquidity_tx.instructions[2].instructions);
//  console.log(decrease_liquidity_tx.instructions[2].instructions[0].keys);
//
//  // Send the transaction
//  signature = await decrease_liquidity_tx.buildAndExecute();
//  console.log("signature:", signature);
//
//  // Wait for the transaction to complete
//  latest_blockhash = await ctx.connection.getLatestBlockhash();
//  await ctx.connection.confirmTransaction({signature, ...latest_blockhash}, "confirmed");
//
//  // Output the liquidity after transaction execution
//  console.log("liquidity(after):", (await positionSDK.refreshData()).liquidity.toString());

  // ********************* CPI DECREASE LIQUIDITY ***********************
  const liquidity = new anchor.BN(11464943);
  const minA = new anchor.BN(0);
  const minB = new anchor.BN(0);
    
  console.log("liquidity to decrease: 11464943");

  try {
      const signature = await program.rpc.decreaseLiquidity(liquidity, minA, minB, {
        accounts: {
          whirlpool: whirlpool,
          position: position.publicKey,
          positionAuthority: userWallet.publicKey,
          positionTokenAccount: userPositionAccount,
          tickArrayLower: tickArrayLower,
          tickArrayUpper: tickArrayUpper,
          tokenOwnerAccountA: userTokenAccountA.address,
          tokenVaultA: tokenVaultA,
          tokenOwnerAccountB: userTokenAccountB.address,
          tokenVaultB: tokenVaultB,
          whirlpoolProgram: orca,
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
  console.log("liquidity(after):", (await positionSDK.refreshData()).liquidity.toString());
  
}

main();
