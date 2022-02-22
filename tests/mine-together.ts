import * as anchor from '@project-serum/anchor';
import fs from 'fs';
import assert from 'assert';
import { expect } from 'chai';
import { PublicKey } from '@solana/web3.js';
import { TOKEN_PROGRAM_ID, Token } from '@solana/spl-token';
import { web3, Program } from '@project-serum/anchor';
import { MineTogether } from '../target/types/mine_together';
import {
  mintToAccount,
  createTokenAccount,
  createTokenMint,
  sleep,
} from './utils';

let program = anchor.workspace.MineTogether as Program<MineTogether>;

const envProvider = anchor.Provider.env();

let provider = envProvider;

function setProvider(p: anchor.Provider) {
  provider = p;
  anchor.setProvider(p);
  program = new anchor.Program(
    program.idl,
    program.programId,
    p
  ) as Program<MineTogether>;
}
setProvider(provider);

describe('mine-together', () => {
  // Aury
  let auryToken: Token;
  let auryMintPubkey: PublicKey;
  let auryVaultPubkey: PublicKey;
  let auryVaultBump: number;

  // Alice
  const alicePubkey = provider.wallet.publicKey;
  let aliceAuryTokenAccount: PublicKey;
  let aliceUserMinerPubkey: PublicKey[] = [];
  let aliceUserMinerBump: number[] = [];

  // Bob
  const bob = web3.Keypair.generate();
  const bobPubkey = bob.publicKey;
  let bobAuryTokenAccount: PublicKey;

  // Config
  let configPubkey: PublicKey;
  let configBump: number;
  const minMineFee = new anchor.BN(1000); // 10%
  const maxMineFee = new anchor.BN(5000); // 50%
  const mineUpdateDelay = new anchor.BN(7); // 7 seconds

  // Mine
  let minePubkey: PublicKey;
  let mineBump: number;
  let mineName = 'Mine-A';
  let mineFee = new anchor.BN(2000); // 20%

  // Miner
  let minerPubkey: PublicKey[] = [];
  let minerBump: number[] = [];
  let minerCount = 2;
  const minerName = [
    'Miner-A',
    'Miner-B',
    'Miner-C',
    'Miner-D',
    'Miner-E',
    'Miner-F',
    'Miner-G',
  ];
  const minerCost = [
    new anchor.BN(10_000_000_000),
    new anchor.BN(20_000_000_000),
    new anchor.BN(30_000_000_000),
    new anchor.BN(40_000_000_000),
    new anchor.BN(50_000_000_000),
    new anchor.BN(60_000_000_000),
    new anchor.BN(70_000_000_000),
  ];
  const minerDuration = [
    new anchor.BN(2),
    new anchor.BN(4),
    new anchor.BN(6),
    new anchor.BN(8),
    new anchor.BN(10),
    new anchor.BN(12),
    new anchor.BN(14),
  ];
  const minerLimit = [
    new anchor.BN(0),
    new anchor.BN(2),
    new anchor.BN(0),
    new anchor.BN(2),
    new anchor.BN(0),
    new anchor.BN(2),
    new anchor.BN(0),
  ];

  describe('Initialize & UpdateConfig', () => {
    it('Prepare Aury', async () => {
      // Aury MintAccount
      const rawData = fs.readFileSync(
        'tests/keys/aury-teST1ieLrLdr4MJPZ7i8mgSCLQ7rTrPRjNnyFdHFaz9.json'
      );
      const keyData = JSON.parse(rawData.toString());
      const mintKey = anchor.web3.Keypair.fromSecretKey(
        new Uint8Array(keyData)
      );

      auryToken = await createTokenMint(
        provider,
        mintKey,
        provider.wallet.publicKey,
        null,
        9,
        TOKEN_PROGRAM_ID
      );
      auryMintPubkey = auryToken.publicKey;

      // Alice
      aliceAuryTokenAccount = await createTokenAccount(
        provider,
        auryMintPubkey,
        alicePubkey
      );
      await mintToAccount(
        provider,
        auryMintPubkey,
        aliceAuryTokenAccount,
        1_000_000_000_000
      );

      // Bob
      await provider.connection.confirmTransaction(
        await provider.connection.requestAirdrop(
          bob.publicKey,
          1 * web3.LAMPORTS_PER_SOL
        ),
        'confirmed'
      );
      bobAuryTokenAccount = await createTokenAccount(
        provider,
        auryMintPubkey,
        bobPubkey
      );

      // Vault
      [auryVaultPubkey, auryVaultBump] =
        await anchor.web3.PublicKey.findProgramAddress(
          [auryMintPubkey.toBuffer()],
          program.programId
        );
    });

    it('Is initialized!', async () => {
      [configPubkey, configBump] =
        await anchor.web3.PublicKey.findProgramAddress(
          [Buffer.from(anchor.utils.bytes.utf8.encode('MINE_TOGETHER_CONFIG'))],
          program.programId
        );

      const minMineFee = new anchor.BN(2000); // 20%
      const maxMineFee = new anchor.BN(3000); // 30%
      const mineUpdateDelay = new anchor.BN(5); // 5 seconds

      await program.rpc.initialize(
        configBump,
        auryVaultBump,
        minMineFee,
        maxMineFee,
        mineUpdateDelay,
        {
          accounts: {
            configAccount: configPubkey,
            auryMint: auryMintPubkey,
            auryVault: auryVaultPubkey,
            initializer: provider.wallet.publicKey,
            systemProgram: anchor.web3.SystemProgram.programId,
            tokenProgram: TOKEN_PROGRAM_ID,
            rent: anchor.web3.SYSVAR_RENT_PUBKEY,
          },
        }
      );

      const configAccount = await program.account.configAccount.fetch(
        configPubkey
      );
      assert.equal(
        configAccount.adminKey.toString(),
        provider.wallet.publicKey.toString()
      );
      assert.equal(configAccount.freezeProgram, false);
      assert.equal(configAccount.minMineFee.toNumber(), minMineFee.toNumber());
      assert.equal(configAccount.maxMineFee.toNumber(), maxMineFee.toNumber());
      assert.equal(
        configAccount.mineUpdateDelay.toNumber(),
        mineUpdateDelay.toNumber()
      );
    });

    it('Update Config Mine', async () => {
      await program.rpc.updateConfigMine(
        configBump,
        minMineFee,
        maxMineFee,
        mineUpdateDelay,
        {
          accounts: {
            configAccount: configPubkey,
            admin: provider.wallet.publicKey,
          },
        }
      );

      const configAccount = await program.account.configAccount.fetch(
        configPubkey
      );
      assert.equal(
        configAccount.adminKey.toString(),
        provider.wallet.publicKey.toString()
      );
      assert.equal(configAccount.freezeProgram, false);
      assert.equal(configAccount.minMineFee.toNumber(), minMineFee.toNumber());
      assert.equal(configAccount.maxMineFee.toNumber(), maxMineFee.toNumber());
      assert.equal(
        configAccount.mineUpdateDelay.toNumber(),
        mineUpdateDelay.toNumber()
      );
    });
  });

  describe('Miner', () => {
    it('Create miner', async () => {
      for (let i = 0; i <= minerCount; i++) {
        const minerCreatedAt = new anchor.BN(Date.now() / 1000);
        const [pubkey, bump] = await anchor.web3.PublicKey.findProgramAddress(
          [
            Buffer.from(
              anchor.utils.bytes.utf8.encode(minerCreatedAt.toString())
            ),
            Buffer.from(anchor.utils.bytes.utf8.encode('MINE_TOGETHER_MINER')),
          ],
          program.programId
        );

        minerPubkey.push(pubkey);
        minerBump.push(bump);

        const [userMinerPubkey, userMinerBump] =
          await anchor.web3.PublicKey.findProgramAddress(
            [
              pubkey.toBuffer(),
              Buffer.from(
                anchor.utils.bytes.utf8.encode('MINE_TOGETHER_MINER')
              ),
              alicePubkey.toBuffer(),
            ],
            program.programId
          );

        aliceUserMinerPubkey.push(userMinerPubkey);
        aliceUserMinerBump.push(userMinerBump);

        await program.rpc.createMiner(
          configBump,
          minerCreatedAt,
          minerBump[i],
          minerName[i],
          minerCost[i],
          minerDuration[i],
          minerLimit[i],
          {
            accounts: {
              configAccount: configPubkey,
              minerAccount: minerPubkey[i],
              admin: provider.wallet.publicKey,
              systemProgram: anchor.web3.SystemProgram.programId,
              rent: anchor.web3.SYSVAR_RENT_PUBKEY,
            },
          }
        );

        const minerAccount = await program.account.minerAccount.fetch(pubkey);
        assert.equal(minerAccount.name, minerName[i]);
        assert.equal(minerAccount.cost.toNumber(), minerCost[i].toNumber());
        assert.equal(
          minerAccount.duration.toNumber(),
          minerDuration[i].toNumber()
        );
        assert.equal(minerAccount.limit.toNumber(), minerLimit[i].toNumber());
        assert.equal(minerAccount.totalPurchased.toNumber(), 0);
        assert.equal(minerAccount.frozenSales, false);

        await sleep(1000);
      }
    });

    it('Remove miner', async () => {
      await program.rpc.removeMiner(configBump, {
        accounts: {
          configAccount: configPubkey,
          minerAccount: minerPubkey[minerCount],
          admin: provider.wallet.publicKey,
        },
      });

      await assert.rejects(
        async () => {
          await program.account.minerAccount.fetch(minerPubkey[minerCount]);
        },
        {
          message:
            'Account does not exist ' + minerPubkey[minerCount].toString(),
        }
      );
    });

    it('Purchase unlimited miner', async () => {
      await program.rpc.purchaseMiner(
        configBump,
        aliceUserMinerBump[0],
        auryVaultBump,
        new anchor.BN(5),
        {
          accounts: {
            configAccount: configPubkey,
            minerAccount: minerPubkey[0],
            userMinerAccount: aliceUserMinerPubkey[0],
            auryMint: auryMintPubkey,
            auryVault: auryVaultPubkey,
            auryFrom: aliceAuryTokenAccount,
            auryFromAuthority: alicePubkey,
            systemProgram: anchor.web3.SystemProgram.programId,
            tokenProgram: TOKEN_PROGRAM_ID,
            rent: anchor.web3.SYSVAR_RENT_PUBKEY,
          },
        }
      );

      const userMinerAccount = await program.account.userMinerAccount.fetch(
        aliceUserMinerPubkey[0]
      );
      expect(userMinerAccount.owner.toString(), alicePubkey.toString());
      expect(userMinerAccount.minerType.toString(), minerPubkey[0].toString());
      expect(userMinerAccount.power.toNumber(), 50_000_000_000);
      expect(userMinerAccount.duration.toNumber(), minerDuration[0].toNumber());
      expect(userMinerAccount.miningStartAt.toNumber(), 0);
      expect(userMinerAccount.mineKey.toString(), PublicKey.default.toString());
      expect(userMinerAccount.xAuryAmount.toNumber(), 0);

      expect(await getTokenBalance(aliceAuryTokenAccount), 950_000_000_000);
      expect(await getTokenBalance(auryVaultPubkey), 50_000_000_000);
    });

    it('Purchase limited miner - failed', async () => {
      await assert.rejects(
        async () => {
          await program.rpc.purchaseMiner(
            configBump,
            aliceUserMinerBump[1],
            auryVaultBump,
            minerLimit[1].add(new anchor.BN(1)),
            {
              accounts: {
                configAccount: configPubkey,
                minerAccount: minerPubkey[1],
                userMinerAccount: aliceUserMinerPubkey[1],
                auryMint: auryMintPubkey,
                auryVault: auryVaultPubkey,
                auryFrom: aliceAuryTokenAccount,
                auryFromAuthority: alicePubkey,
                systemProgram: anchor.web3.SystemProgram.programId,
                tokenProgram: TOKEN_PROGRAM_ID,
                rent: anchor.web3.SYSVAR_RENT_PUBKEY,
              },
            }
          );
        },
        {
          code: 6002,
          message: '6002: Miner purchase limit',
        }
      );
    });

    it('Purchase limited miner - success', async () => {
      await program.rpc.purchaseMiner(
        configBump,
        aliceUserMinerBump[1],
        auryVaultBump,
        minerLimit[1],
        {
          accounts: {
            configAccount: configPubkey,
            minerAccount: minerPubkey[1],
            userMinerAccount: aliceUserMinerPubkey[1],
            auryMint: auryMintPubkey,
            auryVault: auryVaultPubkey,
            auryFrom: aliceAuryTokenAccount,
            auryFromAuthority: alicePubkey,
            systemProgram: anchor.web3.SystemProgram.programId,
            tokenProgram: TOKEN_PROGRAM_ID,
            rent: anchor.web3.SYSVAR_RENT_PUBKEY,
          },
        }
      );

      const userMinerAccount = await program.account.userMinerAccount.fetch(
        aliceUserMinerPubkey[1]
      );
      expect(userMinerAccount.owner.toString(), alicePubkey.toString());
      expect(userMinerAccount.minerType.toString(), minerPubkey[1].toString());
      expect(userMinerAccount.power.toNumber(), 40_000_000_000);
      expect(userMinerAccount.duration.toNumber(), minerDuration[1].toNumber());
      expect(userMinerAccount.miningStartAt.toNumber(), 0);
      expect(userMinerAccount.mineKey.toString(), PublicKey.default.toString());
      expect(userMinerAccount.xAuryAmount.toNumber(), 0);

      expect(await getTokenBalance(aliceAuryTokenAccount), 910_000_000_000);
      expect(await getTokenBalance(auryVaultPubkey), 90_000_000_000);
    });
  });

  describe('Mine', () => {
    it('Create mine', async () => {
      const mineName = 'Mine-AA';
      const mineFee = new anchor.BN(1000);

      [minePubkey, mineBump] = await anchor.web3.PublicKey.findProgramAddress(
        [
          alicePubkey.toBuffer(),
          Buffer.from(anchor.utils.bytes.utf8.encode('MINE_TOGETHER_MINE')),
        ],
        program.programId
      );

      await program.rpc.createMine(configBump, mineBump, mineName, mineFee, {
        accounts: {
          configAccount: configPubkey,
          mineAccount: minePubkey,
          feeTo: aliceAuryTokenAccount,
          owner: alicePubkey,
          systemProgram: anchor.web3.SystemProgram.programId,
          rent: anchor.web3.SYSVAR_RENT_PUBKEY,
        },
      });

      const mineAccount = await program.account.mineAccount.fetch(minePubkey);
      expect(mineAccount.owner.toString(), alicePubkey.toString());
      expect(mineAccount.name, mineName);
      expect(mineAccount.fee.toNumber(), mineFee.toNumber());
      expect(mineAccount.feeTo.toString(), aliceAuryTokenAccount.toString());
      expect(mineAccount.totalAmount.toNumber(), 0);
      expect(mineAccount.xTotalAmount.toNumber(), 0);
      expect(mineAccount.lastUpdatedAt.toNumber(), 0);
      expect(mineAccount.shares.toString(), [].toString());
    });

    it('Create mine again - failed', async () => {
      await assert.rejects(async () => {
        await program.rpc.createMine(configBump, mineBump, mineName, mineFee, {
          accounts: {
            configAccount: configPubkey,
            mineAccount: minePubkey,
            feeTo: aliceAuryTokenAccount,
            owner: alicePubkey,
            systemProgram: anchor.web3.SystemProgram.programId,
            rent: anchor.web3.SYSVAR_RENT_PUBKEY,
          },
        });
      });
    });

    it('Update mine', async () => {
      await program.rpc.updateMine(configBump, mineBump, mineName, mineFee, {
        accounts: {
          configAccount: configPubkey,
          mineAccount: minePubkey,
          feeTo: bobAuryTokenAccount,
          owner: alicePubkey,
        },
      });

      const mineAccount = await program.account.mineAccount.fetch(minePubkey);
      expect(mineAccount.owner.toString(), alicePubkey.toString());
      expect(mineAccount.name, mineName);
      expect(mineAccount.fee.toNumber(), mineFee.toNumber());
      expect(mineAccount.feeTo.toString(), bobAuryTokenAccount.toString());
    });

    it('Update mine owner', async () => {
      await program.rpc.updateMineOwner(mineBump, bobPubkey, {
        accounts: {
          mineAccount: minePubkey,
          owner: alicePubkey,
        },
      });

      const mineAccount = await program.account.mineAccount.fetch(minePubkey);
      expect(mineAccount.owner.toString(), bobPubkey.toString());
    });
  });
});

async function getTokenBalance(pubkey: PublicKey) {
  return parseInt(
    (await provider.connection.getTokenAccountBalance(pubkey)).value.amount
  );
}
