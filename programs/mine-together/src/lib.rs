pub mod utils;

use crate::utils::*;
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
    pub fn toggle_freeze_program(ctx: Context<FreezeProgram>, _nonce_staking: u8) -> ProgramResult {
        ctx.accounts.mine_together_account.freeze_program =
            !ctx.accounts.mine_together_account.freeze_program;

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

        // update the miner
        miner.index = mine_together.miner_counter;
        miner.cost = cost;
        miner.duration = duration;

        // update the mine together
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

    pub fn purchase_miner(
        ctx: Context<PurchaseMiner>,
        _nonce_mine_together: u8,
        _nonce_miner: u8,
        _nonce_aury_vault: u8,
    ) -> ProgramResult {
        let miner = &mut ctx.accounts.miner_account;
        let aury_vault = &mut ctx.accounts.aury_vault;
        let aury_from = &mut ctx.accounts.aury_from;
        let aury_from_authority = &ctx.accounts.aury_from_authority;
        let token_program = &ctx.accounts.token_program;

        // transfer aury to the vault
        spl_token_transfer(TokenTransferParams {
            source: aury_from.to_account_info(),
            destination: aury_vault.to_account_info(),
            amount: miner.cost,
            authority: aury_from_authority.to_account_info(),
            authority_signer_seeds: &[],
            token_program: token_program.to_account_info(),
        })?;

        // update the miner
        miner.owner = *aury_from_authority.key;

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
#[instruction(_nonce_mine_together: u8)]
pub struct FreezeProgram<'info> {
    #[account(
        mut,
        seeds = [ constants::MINE_TOGETHER_PDA_SEED.as_ref() ],
        bump = _nonce_mine_together,
    )]
    pub mine_together_account: Box<Account<'info, MineTogetherAccount>>,

    pub admin: Signer<'info>,
}

#[derive(Accounts)]
#[instruction(_nonce_mine_together: u8, _nonce_miner: u8)]
pub struct CreateMiner<'info> {
    #[account(
        mut,
        seeds = [ constants::MINE_TOGETHER_PDA_SEED.as_ref() ],
        bump = _nonce_mine_together,
        constraint = !mine_together_account.freeze_program @ ErrorCode::ProgramFreezed
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
        constraint = !mine_together_account.freeze_program @ ErrorCode::ProgramFreezed
    )]
    pub mine_together_account: Box<Account<'info, MineTogetherAccount>>,

    #[account(
        mut,
        close = admin,
        seeds = [ mine_together_account.miner_counter.to_string().as_ref(), constants::MINER_PDA_SEED.as_ref() ],
        bump = _nonce_miner,
        constraint = miner_account.owner != Pubkey::default() @ ErrorCode::MinerPurhcased,
    )]
    pub miner_account: Box<Account<'info, MinerAccount>>,

    pub admin: Signer<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(_nonce_mine_together: u8, _nonce_miner: u8, _nonce_aury_vault: u8)]
pub struct PurchaseMiner<'info> {
    #[account(
        seeds = [ constants::MINE_TOGETHER_PDA_SEED.as_ref() ],
        bump = _nonce_mine_together,
        constraint = !mine_together_account.freeze_program @ ErrorCode::ProgramFreezed
    )]
    pub mine_together_account: Box<Account<'info, MineTogetherAccount>>,

    #[account(
        mut,
        seeds = [ mine_together_account.miner_counter.to_string().as_ref(), constants::MINER_PDA_SEED.as_ref() ],
        bump = _nonce_miner,
        constraint = miner_account.owner != Pubkey::default() @ ErrorCode::MinerPurhcased,
    )]
    pub miner_account: Box<Account<'info, MinerAccount>>,

    #[account(
        address = constants::AURY_TOKEN_MINT_PUBKEY.parse::<Pubkey>().unwrap(),
    )]
    pub aury_mint: Box<Account<'info, Mint>>,

    #[account(
        mut,
        seeds = [ constants::AURY_TOKEN_MINT_PUBKEY.parse::<Pubkey>().unwrap().as_ref() ],
        bump = _nonce_aury_vault,
    )]
    pub aury_vault: Box<Account<'info, TokenAccount>>,

    #[account(mut)]
    pub aury_from: Box<Account<'info, TokenAccount>>,

    pub aury_from_authority: Signer<'info>,

    pub token_program: Program<'info, Token>,
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
}

#[error]
pub enum ErrorCode {
    #[msg("Not admin")]
    NotAdmin, // 6000, 0x1770
    #[msg("Program freezed")]
    ProgramFreezed, // 6001, 0x1771
    #[msg("Miner is already purchased")]
    MinerPurhcased, // 6002, 0x1772
    #[msg("Token transfer failed")]
    TokenTransferFailed, // 6003, 0x1773
}
