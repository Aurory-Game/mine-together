import * as anchor from '@project-serum/anchor';
import { Program } from '@project-serum/anchor';
import { MineTogether } from '../target/types/mine_together';

describe('mine-together', () => {

  // Configure the client to use the local cluster.
  anchor.setProvider(anchor.Provider.env());

  const program = anchor.workspace.MineTogether as Program<MineTogether>;

  it('Is initialized!', async () => {
    // Add your test here.
    const tx = await program.rpc.initialize({});
    console.log("Your transaction signature", tx);
  });
});
