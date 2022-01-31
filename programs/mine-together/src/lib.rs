use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token, TokenAccount};

declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

#[cfg(not(feature = "local-testing"))]
pub mod constants {
    pub const AURY_TOKEN_MINT_PUBKEY: &str = "AURYydfxJib1ZkTir1Jn1J9ECYUtjb6rKQVmtYaixWPP";
    pub const MINE_TOGETHER_PDA_SEED: &[u8] = b"MINE_TOGETHER";
    pub const MINER_PDA_SEED: &[u8] = b"MINE_TOGETHER_MINER";
}

#[cfg(feature = "local-testing")]
pub mod constants {
    pub const AURY_TOKEN_MINT_PUBKEY: &str = "teST1ieLrLdr4MJPZ7i8mgSCLQ7rTrPRjNnyFdHFaz9";
    pub const MINE_TOGETHER_PDA_SEED: &[u8] = b"MINE_TOGETHER";
    pub const MINER_PDA_SEED: &[u8] = b"MINE_TOGETHER_MINER";
}

#[program]
pub mod mine_together {
    use super::*;

    pub fn initialize(
        ctx: Context<Initialize>,
        _nonce_mine_together: u8,
        _nonce_aury_vault: u8,
    ) -> ProgramResult {
        ctx.accounts.mine_together_account.admin_key = *ctx.accounts.initializer.key;

        Ok(())
    }

    #[access_control(ctx.accounts.mine_together_account.assert_admin(&ctx.accounts.admin))]
    pub fn create_miner(
        ctx: Context<CreateMiner>,
        _nonce_mine_together: u8,
        _nonce_miner: u8,
        cost: u64,
        duration: u64,
    ) -> ProgramResult {
        let mine_together = &mut ctx.accounts.mine_together_account;
        let miner = &mut ctx.accounts.miner_account;

        miner.index = mine_together.miner_counter;
        miner.cost = cost;
        miner.duration = duration;

        mine_together.miner_counter += 1;

        Ok(())
    }

    #[access_control(ctx.accounts.mine_together_account.assert_admin(&ctx.accounts.admin))]
    pub fn remove_miner(
        ctx: Context<RemoveMiner>,
        _nonce_mine_together: u8,
        _nonce_miner: u8,
    ) -> ProgramResult {
        Ok(())
    }
}

#[derive(Accounts)]
#[instruction(_nonce_mine_together: u8, _nonce_aury_vault: u8)]
pub struct Initialize<'info> {
    #[account(
        init,
        payer = initializer,
        seeds = [ constants::MINE_TOGETHER_PDA_SEED.as_ref() ],
        bump = _nonce_mine_together,
    )]
    pub mine_together_account: Box<Account<'info, MineTogetherAccount>>,

    #[account(
        address = constants::AURY_TOKEN_MINT_PUBKEY.parse::<Pubkey>().unwrap(),
    )]
    pub aury_mint: Box<Account<'info, Mint>>,

    #[account(
        init,
        payer = initializer,
        token::mint = aury_mint,
        token::authority = aury_vault,
        seeds = [ constants::AURY_TOKEN_MINT_PUBKEY.parse::<Pubkey>().unwrap().as_ref() ],
        bump = _nonce_aury_vault,
    )]
    pub aury_vault: Box<Account<'info, TokenAccount>>,

    #[account(mut)]
    pub initializer: Signer<'info>,

    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
#[instruction(_nonce_mine_together: u8, _nonce_miner: u8)]
pub struct CreateMiner<'info> {
    #[account(
        mut,
        seeds = [ constants::MINE_TOGETHER_PDA_SEED.as_ref() ],
        bump = _nonce_mine_together,
    )]
    pub mine_together_account: Box<Account<'info, MineTogetherAccount>>,

    #[account(
        init,
        payer = admin,
        seeds = [ mine_together_account.miner_counter.to_string().as_ref(), constants::MINER_PDA_SEED.as_ref() ],
        bump = _nonce_miner,
    )]
    pub miner_account: Box<Account<'info, MinerAccount>>,

    pub admin: Signer<'info>,

    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
#[instruction(_nonce_mine_together: u8, _nonce_miner: u8)]
pub struct RemoveMiner<'info> {
    #[account(
        mut,
        seeds = [ constants::MINE_TOGETHER_PDA_SEED.as_ref() ],
        bump = _nonce_mine_together,
    )]
    pub mine_together_account: Box<Account<'info, MineTogetherAccount>>,

    #[account(
        mut,
        close = admin,
        seeds = [ mine_together_account.miner_counter.to_string().as_ref(), constants::MINER_PDA_SEED.as_ref() ],
        bump = _nonce_miner,
        constraint = miner_account.x_aury_amount == 0,
    )]
    pub miner_account: Box<Account<'info, MinerAccount>>,

    pub admin: Signer<'info>,

    pub system_program: Program<'info, System>,
}

#[account]
#[derive(Default)]
pub struct MineTogetherAccount {
    pub admin_key: Pubkey,
    pub freeze_program: bool,
    pub mine_counter: u32,
    pub miner_counter: u32,
}

#[account]
#[derive(Default)]
pub struct MineAccount {
    pub index: u32,
    pub fee: u64,
    pub total_amount: u64,
    pub x_total_amount: u64,
}

#[account]
#[derive(Default)]
pub struct MinerAccount {
    pub index: u32,
    pub cost: u64,
    pub duration: u64,
    pub mining_start_at: u64,
    pub owner: Pubkey,
    pub x_aury_amount: u64,
}

impl MineTogetherAccount {
    pub fn assert_admin(&self, signer: &Signer) -> ProgramResult {
        if self.admin_key != *signer.key {
            return Err(ErrorCode::NotAdmin.into());
        }

        Ok(())
    }

    pub fn assert_freeze_program(&self) -> ProgramResult {
        if self.freeze_program {
            return Err(ErrorCode::ProgramFreezed.into());
        }

        Ok(())
    }
}

#[error]
pub enum ErrorCode {
    #[msg("Not admin")]
    NotAdmin, // 6000, 0x1770
    #[msg("Program freezed")]
    ProgramFreezed, // 6001, 0x1771
    #[msg("Token transfer failed")]
    TokenTransferFailed, // 6002, 0x1772
}
