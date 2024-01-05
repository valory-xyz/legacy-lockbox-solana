import { PublicKey } from "@solana/web3.js";
import { AnchorProvider, BN } from "@coral-xyz/anchor";
import { DecimalUtil, Percentage } from "@orca-so/common-sdk";
import {
  WhirlpoolContext, buildWhirlpoolClient, ORCA_WHIRLPOOL_PROGRAM_ID,
  PDAUtil, swapQuoteByInputToken, IGNORE_CACHE, SwapUtils
} from "@orca-so/whirlpools-sdk";
import { adjustForSlippage } from "@orca-so/whirlpools-sdk/dist/utils/position-util";
import Decimal from "decimal.js";
import * as assert from "assert";

// Environment variables must be defined before script execution
// ANCHOR_PROVIDER_URL=https://api.mainnet-beta.solana.com
// ANCHOR_WALLET=wallet.json

async function main() {
    if (process.argv.length < 4) {
      console.error('Expected at least one argument!');
      process.exit(1);
    }

    let execute = false;

    // Set the trade amount in
    let amount = "0";
    if (process.argv[2] && process.argv[2] === '-a' && process.argv[3]) {
      amount = process.argv[3];
    }

    // Set the execute flag
    if (process.argv[4] && process.argv[4] === '-e') {
      execute = true;
    }

  if (amount === "0") {
    console.error("Amount must be bigger than zero");
    process.exit(1);
  }

  // Create WhirlpoolClient
  const provider = AnchorProvider.env();
  const ctx = WhirlpoolContext.withProvider(provider, ORCA_WHIRLPOOL_PROGRAM_ID);
  const client = buildWhirlpoolClient(ctx);

  console.log("endpoint:", ctx.connection.rpcEndpoint);
  console.log("wallet pubkey:", ctx.wallet.publicKey.toBase58());

  // Token definition
  // https://everlastingsong.github.io/nebula/
  const olas = {mint: new PublicKey("Ez3nzG9ofodYCvEmw73XhQ87LWNYVRM2s7diB5tBZPyM"), decimals: 8};
  const sol = {mint: new PublicKey("So11111111111111111111111111111111111111112"), decimals: 9};

  // WhirlpoolsConfig account

  // Get sol/olas whirlpool
  // Whirlpools are identified by 5 elements (Program, Config, mint address of the 1st token,
  // mint address of the 2nd token, tick spacing), similar to the 5 column compound primary key in DB
  const tick_spacing = 64;
  const whirlpool_pubkey = new PublicKey("5dMKUYJDsjZkAD3wiV3ViQkuq9pSmWQ5eAzcQLtDnUT3");
  console.log("whirlpool_key:", whirlpool_pubkey.toBase58());
  const whirlpool = await client.getPool(whirlpool_pubkey);

  // Swap 1 sol for olas
  //const amount_in = new BN(10000);
  const amount_in = new Decimal(amount /* sol */);
  const tradeAmount = DecimalUtil.toBN(amount_in, olas.decimals);
  console.log("tradeAmount", tradeAmount.toNumber());

  // Slippage
  let slippageTolerance = Percentage.fromFraction(10, 1000);

  // Obtain swap estimation (run simulation)
  const quote = await swapQuoteByInputToken(
    whirlpool,
    // Input token and amount
    olas.mint,
    tradeAmount,
    // Acceptable slippage (10/1000 = 1%)
    slippageTolerance,
    ctx.program.programId,
    ctx.fetcher,
    IGNORE_CACHE,
  );

  //console.log("Quote", quote);

  // Output the estimation
  const amountIn = DecimalUtil.fromBN(quote.estimatedAmountIn, olas.decimals);
  const amountOut = DecimalUtil.fromBN(quote.estimatedAmountOut, sol.decimals);
  console.log("estimatedAmountIn:", amountIn.toString(), "olas");
  console.log("estimatedAmountOut:", amountOut.toString(), "sol");
  console.log("otherAmountThreshold:", DecimalUtil.fromBN(quote.otherAmountThreshold, sol.decimals).toString(), "sol");
  const sol2olas = amountOut.toNumber() / amountIn.toNumber();
  const olas2sol = amountIn.toNumber() / amountOut.toNumber();
  console.log("rate SOL => OLAS:", sol2olas);
  console.log("rate OLAS => SOL:", olas2sol);

    // Verify with an actual swap.
    assert.equal(quote.aToB, false);
    assert.equal(quote.amountSpecifiedIsInput, true);
    assert.equal(
      quote.sqrtPriceLimit.toString(),
      SwapUtils.getDefaultSqrtPriceLimit(false).toString()
    );
    assert.equal(
      quote.otherAmountThreshold.toString(),
      adjustForSlippage(quote.estimatedAmountOut, slippageTolerance, false).toString()
    );
    assert.equal(quote.estimatedAmountIn.toString(), tradeAmount);

    if (execute) {
      assert.doesNotThrow(async () => await (await whirlpool.swap(quote)).buildAndExecute());
    }
}

main();
