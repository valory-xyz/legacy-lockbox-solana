#!/bin/bash
# networks lists: https://solana.com/rpc
# NETWORK=localhost
# NETWORK=https://api.devnet.solana.com
NETWORK=https://api.mainnet-beta.solana.com
# NETWORK=https://api.testnet.solana.com
# progmramId
# result of generation
PPKEYFILE=$1
PD=$2

if [ ! -f ${PPKEYFILE} ]; then
	echo "missing ${PPKEYFILE}"
	exit 1
fi

# JSON key file (Keypair Path) available via the "solana config get" command
WALLET=$3
WALLETK=$(solana address -k ${WALLET})

# configure to deploy
solana config set --keypair ${WALLET}
solana config set --url ${NETWORK}
#solana airdrop 10
solana balance ${WALLETK} -u ${NETWORK}

## We need to make absolute sure the solana version matches the cluster version
# https://solana.stackexchange.com/questions/4083/blockhash-expired-5-retries-remaining
v1=$(solana --version)
v2=$(solana cluster-version)
if [[ "${v1}" =~ .*"${v2}".* ]]; then
	echo "solana version is OK"
else
	echo "solana version mismatch. deploy not possible. long life to this blockchain!"
	echo "details: https://solana.stackexchange.com/questions/4083/blockhash-expired-5-retries-remaining"
        # dirty fix: sh -c $(curl -sSfL https://release.solana.com/v1.14.19/install)
	exit 1
fi

# deploy
# Required balance: (6.331686 SOL) + fee (0.00227 SOL)
solana program deploy --url ${NETWORK} -v --program-id ${PPKEYFILE} liquidity_lockbox.so
solana balance ${PD} -u ${NETWORK}
solana program show ${PD}
