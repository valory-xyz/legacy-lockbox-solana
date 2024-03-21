# Deployment
Mak sure the Solana config is set for the mainnet. See the Solana configuration using the following command:
```
solana config get
```

Run the deployment script:
```
./deploy.sh program_keypair.json program_id path_to_deployer_key.json
```

where `program_keypair.json` is the keypair for the deployed program, `program_id` is the program ID corresponding to
the `program_keypair`, and `path_to_deployer_key.json` is the deployer keypair path obtained using the `solana config get`
command.

Then run the initialization script:
```
npx ts-node lockbox_init.ts
```

To add liquidity:
