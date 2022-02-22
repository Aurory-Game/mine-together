import { Keypair, PublicKey, SystemProgram } from '@solana/web3.js';
import { MintLayout, TOKEN_PROGRAM_ID } from '@solana/spl-token';
import { Token } from '@solana/spl-token';
import { TokenInstructions } from '@project-serum/serum';
import { web3, Provider } from '@project-serum/anchor';

const Transaction = web3.Transaction;

export function sleep(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

export async function mintToAccount(
  provider: Provider,
  mint: PublicKey,
  destination: PublicKey,
  amount: number
) {
  const tx = new Transaction();
  tx.add(
    Token.createMintToInstruction(
      TOKEN_PROGRAM_ID,
      mint,
      destination,
      provider.wallet.publicKey,
      [],
      amount
    )
  );
  await provider.send(tx);
}

export async function createTokenAccount(
  provider: Provider,
  mint: PublicKey,
  owner: PublicKey
) {
  const vault = Keypair.generate();
  const tx = new Transaction();
  tx.add(
    ...(await createTokenAccountInstrs(provider, vault.publicKey, mint, owner))
  );
  await provider.send(tx, [vault]);
  return vault.publicKey;
}

export async function createTokenAccountInstrs(
  provider: Provider,
  newAccountPubkey: PublicKey,
  mint: PublicKey,
  owner: PublicKey,
  lamports: number | undefined = undefined
) {
  if (lamports === undefined) {
    lamports = await provider.connection.getMinimumBalanceForRentExemption(165);
  }
  return [
    SystemProgram.createAccount({
      fromPubkey: provider.wallet.publicKey,
      newAccountPubkey,
      space: 165,
      lamports,
      programId: TOKEN_PROGRAM_ID,
    }),
    TokenInstructions.initializeAccount({
      account: newAccountPubkey,
      mint,
      owner,
    }),
  ];
}

export async function createTokenMint(
  provider: Provider,
  mintAccount: Keypair,
  mintAuthority: PublicKey,
  freezeAuthority: PublicKey | null,
  decimals: number,
  programId: PublicKey
) {
  const payer = web3.Keypair.generate();

  //airdrop tokens
  await provider.connection.confirmTransaction(
    await provider.connection.requestAirdrop(
      payer.publicKey,
      1 * web3.LAMPORTS_PER_SOL
    ),
    'confirmed'
  );

  const token = new Token(
    provider.connection,
    mintAccount.publicKey,
    programId,
    payer
  );

  // Allocate memory for the account
  const balanceNeeded = await Token.getMinBalanceRentForExemptMint(
    provider.connection
  );

  const transaction = new web3.Transaction();
  transaction.add(
    web3.SystemProgram.createAccount({
      fromPubkey: provider.wallet.publicKey,
      newAccountPubkey: mintAccount.publicKey,
      lamports: balanceNeeded,
      space: MintLayout.span,
      programId,
    })
  );

  transaction.add(
    Token.createInitMintInstruction(
      programId,
      mintAccount.publicKey,
      decimals,
      mintAuthority,
      freezeAuthority
    )
  );

  await provider.send(transaction, [mintAccount]);
  return token;
}
