import * as idl from "../target/idl/liquidity_lockbox.json";
import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { LiquidityLockbox } from "../target/types/liquidity_lockbox";
import {
  createMint, mintTo, transfer, getOrCreateAssociatedTokenAccount,
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
      const slippage = Percentage.fromFraction(10, 1000); // 1%
      // Full range price
      const tickSpacing = 64;
      const [lower_tick_index, upper_tick_index] = TickUtil.getFullRangeTickIndex(tickSpacing);
      //const lower_tick_index = -443584;
      //const upper_tick_index = 443584;

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
        inputTokenMint: olas,
        inputTokenAmount: olas_amount,
        // Acceptable slippage
        slippageTolerance: slippage,
      });

      // Output the estimation
      console.log("SOL max input:", DecimalUtil.fromBN(quote.tokenMaxA, token_a.decimals).toFixed(token_a.decimals));
      console.log("OLAS max input:", DecimalUtil.fromBN(quote.tokenMaxB, token_b.decimals).toFixed(token_b.decimals));

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

    // Find a PDA account for the program
    const [pdaProgram, bump] = await anchor.web3.PublicKey.findProgramAddress([Buffer.from("liquidity_lockbox", "utf-8")], program.programId);
    const bumpBytes = Buffer.from(new Uint8Array([bump]));
    console.log("Lockbox PDA:", pdaProgram.toBase58());

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

    // Initialize the LiquidityLockbox state
    try {
        signature = await program.methods
          .initialize()
          .accounts(
            {
              bridgedTokenMint: bridgedTokenMint,
              feeCollectorTokenOwnerAccountA: feeCollectorTokenOwnerAccountA.address,
              feeCollectorTokenOwnerAccountB: feeCollectorTokenOwnerAccountB.address,
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

    // Try to initialize the LiquidityLockbox state
    try {
        signature = await program.methods
          .initialize()
          .accounts(
            {
              bridgedTokenMint: bridgedTokenMint,
              feeCollectorTokenOwnerAccountA: feeCollectorTokenOwnerAccountA.address,
              feeCollectorTokenOwnerAccountB: feeCollectorTokenOwnerAccountB.address,
            }
          )
          .rpc();
    } catch (error) {}

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
    accountInfo = await provider.connection.getAccountInfo(positionMint);
    //console.log(accountInfo);

    // Get the ATA of the userWallet address, and if it does not exist, create it
    // This account has an NFT token
    const positionTokenAccount = parsed.address;
    console.log("User ATA for NFT:", positionTokenAccount.toBase58());

    // Get the ATA of the userWallet address, and if it does not exist, create it
    // This account will have bridged tokens
    const bridgedTokenAccount = await getOrCreateAssociatedTokenAccount(
        provider.connection,
        userWallet,
        bridgedTokenMint,
        userWallet.publicKey
    );
    console.log("User ATA for bridged:", bridgedTokenAccount.address.toBase58());

//    accountInfo = await provider.connection.getAccountInfo(positionTokenAccount);
//    console.log(accountInfo);

//    let balance = await program.methods.getBalance()
//      .accounts({account: positionTokenAccount})
//      .view();
//    console.log("User ATA must have one NFT, balance:", balance.toNumber());

    // ATA for the PDA to store the position NFT
    const pdaPositionAccount = await getAssociatedTokenAddress(
        positionMint,
        pdaProgram,
        true // allowOwnerOffCurve - allow pda accounts to be have associated token account
    );
    console.log("PDA ATA", pdaPositionAccount.toBase58());

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
//  decrease_liquidity_tx.instructions[2].instructions[0].keys[5].pubkey = tokenOwnerAccountA.address;
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

    // ############################## DEPOSIT ##############################
    console.log("\nSending position NFT to the program in exchange of bridged tokens");

//    const positionDataAccount = anchor.web3.Keypair.generate();

    // Create pseudo-position corresponding to the NFT
//    await positionProgram.methods
//      .new(whirlpool, positionMint)
//      .accounts({ dataAccount: positionDataAccount.publicKey })
//      .signers([positionDataAccount])
//      .rpc();

//    accountInfo = await provider.connection.getAccountInfo(pdaPositionAccount);
//    console.log(accountInfo);

    // Get the state data
    let lockboxStateData = await program.account.liquidityLockbox.fetch(pdaProgram);
    const numPosition = lockboxStateData.numPositions;

    // Find a PDA account for the lockbox position
    const bytesStr = Buffer.from("lockbox_position", "utf-8");
    const bytesNum = Buffer.allocUnsafe(4);
    bytesNum.writeInt32LE(numPosition);
    const [pdaLockboxPosition, positionBump] = await anchor.web3.PublicKey.findProgramAddress([bytesStr, bytesNum], program.programId);
    const positionBumpBytes = Buffer.from(new Uint8Array([positionBump]));
    console.log("PDA Lockbox Position:", pdaLockboxPosition.toBase58());

    // Try to pass another user ATA with a mint that is different from the position mint
    try {
        signature = await program.methods.deposit(numPosition)
          .accounts(
              {
                lockbox: pdaProgram,
                positionTokenAccount: tokenOwnerAccountA.address,
                pdaPositionAccount: pdaPositionAccount,
                pdaLockboxPosition: pdaLockboxPosition,
                bridgedTokenAccount: bridgedTokenAccount.address,
                bridgedTokenMint: bridgedTokenMint,
                position: position.publicKey,
              }
          )
          .signers([userWallet])
          .rpc();
    } catch (error) {}

    // Try to pass user position ATA instead of the PDA position ATA
    try {
        signature = await program.methods.deposit(numPosition)
          .accounts(
              {
                lockbox: pdaProgram,
                positionTokenAccount: positionTokenAccount,
                pdaPositionAccount: positionTokenAccount,
                pdaLockboxPosition: pdaLockboxPosition,
                bridgedTokenAccount: bridgedTokenAccount.address,
                bridgedTokenMint: bridgedTokenMint,
                position: position.publicKey
              }
          )
          .signers([userWallet])
          .rpc();
    } catch (error) {}

    //getAssociatedTokenAddressSync(positionMint, pdaProgram)
    // Execute the correct deposit tx
    try {
        signature = await program.methods.deposit(numPosition)
          .accounts(
              {
                lockbox: pdaProgram,
                positionTokenAccount: positionTokenAccount,
                pdaPositionAccount: pdaPositionAccount,
                positionMint: positionMint,
                pdaLockboxPosition: pdaLockboxPosition,
                bridgedTokenAccount: bridgedTokenAccount.address,
                bridgedTokenMint: bridgedTokenMint,
                position: position.publicKey
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

    console.log("Deposit tx signature", signature);
    // Wait for program creation confirmation
    await provider.connection.confirmTransaction({
        signature: signature,
        ...(await provider.connection.getLatestBlockhash()),
    });

  let tokenAccounts = await provider.connection.getTokenAccountsByOwner(
    pdaProgram,
    { programId: TOKEN_PROGRAM_ID }
  );

  tokenAccounts.value.forEach((tokenAccount) => {
    const accountData = AccountLayout.decode(tokenAccount.account.data);
    if (accountData.mint.toString() === positionMint.toString()) {
      console.log("PDA ATA is transferred the NFT, balance:", accountData.amount.toString());
    }
  });

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

  lockboxStateData = await program.account.liquidityLockbox.fetch(pdaProgram);
  expect(data.liquidity.toString()).toEqual(lockboxStateData.totalLiquidity.toString());

//    balance = await program.methods.getBalance()
//      .accounts({account: pdaPositionAccount.address})
//      .view();
//    console.log("PDA ATA is transferred the NFT, balance:", balance.toNumber());
//
//    balance = await program.methods.getBalance()
//      .accounts({account: positionTokenAccount})
//      .view();
//    console.log("User ATA NFT balance now:", balance.toNumber());
//
//    balance = await program.methods.getBalance()
//      .accounts({account: bridgedTokenAccount.address})
//      .view();
//    console.log("User ATA bridged balance now:", balance.toNumber());
//    expect(data.liquidity.toNumber()).toEqual(balance.toNumber());
//
//    let totalSupply = await program.methods.totalSupply()
//      .accounts({account: bridgedTokenMint})
//      .view();
//    console.log("Bridged token total supply now:", totalSupply.toNumber());

    // ############################## WITHDRAW ##############################
    console.log("\nSending bridged tokens back to the program in exchange of the liquidity split in both tokens");

    const zeroAmount = new anchor.BN("0");
    const bigBalance = new anchor.BN("4000000000");
    // Try to get amounts and positions for a bigger provided liquidity amount than the total liquidity
    try {
        signature = await program.methods.withdraw(numPosition, bigBalance, zeroAmount, zeroAmount)
          .accounts(
              {
                lockbox: pdaProgram,
                whirlpoolProgram: orca,
                whirlpool: whirlpool,
                tokenProgram: TOKEN_PROGRAM_ID,
                position: position.publicKey,
                positionMint: positionMint,
                pdaLockboxPosition: pdaLockboxPosition,
                bridgedTokenAccount: bridgedTokenAccount.address,
                bridgedTokenMint: bridgedTokenMint,
                pdaPositionAccount: pdaPositionAccount,
                tokenOwnerAccountA: tokenOwnerAccountA.address,
                tokenOwnerAccountB: tokenOwnerAccountB.address,
                feeCollectorTokenOwnerAccountA: feeCollectorTokenOwnerAccountA.address,
                feeCollectorTokenOwnerAccountB: feeCollectorTokenOwnerAccountB.address,
                tokenVaultA: tokenVaultA,
                tokenVaultB: tokenVaultB,
                tickArrayLower: tickArrayLower,
                tickArrayUpper: tickArrayUpper
              }
          )
          .signers([userWallet])
          .rpc();
    } catch (error) {}

    // Transfer bridged tokens from the user to the program, decrease the position and send tokens back to the user
    const tBalalnce = data.liquidity;//new anchor.BN("20000000");

    // Try to execute the withdraw with the incorrect position address
    try {
        signature = await program.methods.withdraw(numPosition, tBalalnce, zeroAmount, zeroAmount)
          .accounts(
              {
                lockbox: pdaProgram,
                whirlpoolProgram: orca,
                whirlpool: whirlpool,
                tokenProgram: TOKEN_PROGRAM_ID,
                position: bridgedTokenAccount.address,
                positionMint: positionMint,
                pdaLockboxPosition: pdaLockboxPosition,
                bridgedTokenAccount: bridgedTokenAccount.address,
                bridgedTokenMint: bridgedTokenMint,
                pdaPositionAccount: pdaPositionAccount,
                tokenOwnerAccountA: tokenOwnerAccountA.address,
                tokenOwnerAccountB: tokenOwnerAccountB.address,
                feeCollectorTokenOwnerAccountA: feeCollectorTokenOwnerAccountA.address,
                feeCollectorTokenOwnerAccountB: feeCollectorTokenOwnerAccountB.address,
                tokenVaultA: tokenVaultA,
                tokenVaultB: tokenVaultB,
                tickArrayLower: tickArrayLower,
                tickArrayUpper: tickArrayUpper
              }
          )
          .signers([userWallet])
          .rpc();
    } catch (error) {}

    lockboxStateData = await program.account.liquidityLockbox.fetch(pdaProgram);
    console.log("numPositions", lockboxStateData.numPositions);

    accountInfo = await provider.connection.getAccountInfo(pdaLockboxPosition);
    //console.log(accountInfo);

    // Recover all the necessary data from the corresponding data account
    let positionStateData = await program.account.lockboxPosition.fetch(pdaLockboxPosition);
    // Check that accounts match
    expect(positionStateData.positionAccount.toString()).toEqual(position.publicKey.toString());
    expect(positionStateData.positionPdaAta.toString()).toEqual(pdaPositionAccount.toString());
    expect(positionStateData.positionLiquidity.toString()).toEqual(data.liquidity.toString());

    // Execute the correct withdraw tx
    console.log("Amount of bridged tokens to withdraw:", tBalalnce.toString());
    try {
        signature = await program.methods.withdraw(numPosition, tBalalnce, zeroAmount, zeroAmount)
          .accounts(
              {
                lockbox: pdaProgram,
                whirlpoolProgram: orca,
                whirlpool: whirlpool,
                tokenProgram: TOKEN_PROGRAM_ID,
                position: position.publicKey,
                positionMint: positionMint,
                pdaLockboxPosition: pdaLockboxPosition,
                bridgedTokenAccount: bridgedTokenAccount.address,
                bridgedTokenMint: bridgedTokenMint,
                pdaPositionAccount: pdaPositionAccount,
                tokenOwnerAccountA: tokenOwnerAccountA.address,
                tokenOwnerAccountB: tokenOwnerAccountB.address,
                feeCollectorTokenOwnerAccountA: feeCollectorTokenOwnerAccountA.address,
                feeCollectorTokenOwnerAccountB: feeCollectorTokenOwnerAccountB.address,
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

  lockboxStateData = await program.account.liquidityLockbox.fetch(pdaProgram);
  console.log("Liquidity now:", lockboxStateData.totalLiquidity.toString());

  accountInfo = await provider.connection.getAccountInfo(pdaLockboxPosition);
  //console.log(accountInfo);

//    balance = await program.methods.getBalance()
//      .accounts({account: bridgedTokenAccount.address})
//      .view();
//    console.log("User ATA bridged balance now:", balance.toNumber());
//
//    balance = await program.methods.getBalance()
//      .accounts({account: pdaPositionAccount.address})
//      .view();
//    console.log("PDA ATA NFT balance now:", balance.toNumber());
//
//    totalSupply = await program.methods.totalSupply()
//      .accounts({account: bridgedTokenMint})
//      .view();
//    console.log("Bridged token total supply now:", totalSupply.toNumber());
}

main();
