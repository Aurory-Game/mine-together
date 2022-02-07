pub mod utils;

use crate::utils::*;
use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token, TokenAccount};
use std::convert::TryInto;

declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

#[cfg(not(feature = "local-testing"))]
pub mod constants {
    pub const AURY_TOKEN_MINT_PUBKEY: &str = "AURYydfxJib1ZkTir1Jn1J9ECYUtjb6rKQVmtYaixWPP";
    pub const MINE_TOGETHER_PDA_SEED: &[u8] = b"MINE_TOGETHER";
    pub const MINE_PDA_SEED: &[u8] = b"MINE_TOGETHER_MINE";
    pub const MINER_PDA_SEED: &[u8] = b"MINE_TOGETHER_MINER";
}

#[cfg(feature = "local-testing")]
pub mod constants {
    pub const AURY_TOKEN_MINT_PUBKEY: &str = "teST1ieLrLdr4MJPZ7i8mgSCLQ7rTrPRjNnyFdHFaz9";
    pub const MINE_TOGETHER_PDA_SEED: &[u8] = b"MINE_TOGETHER";
    pub const MINE_PDA_SEED: &[u8] = b"MINE_TOGETHER_MINE";
    pub const MINER_PDA_SEED: &[u8] = b"MINE_TOGETHER_MINER";
}

#[program]
pub mod mine_together {
    use super::*;

    pub const FEE_MULTIPLIER: u64 = 10000; // 100%
    pub const MAX_MINE_FEE: u64 = 2000; // 20%
    pub const MIN_MINE_FEE: u64 = 0; // 0%

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
        _miner_index: u32,
        _nonce_miner: u8,
    ) -> ProgramResult {
        Ok(())
    }

    pub fn purchase_miner(
        ctx: Context<PurchaseMiner>,
        _nonce_mine_together: u8,
        _miner_index: u32,
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

    pub fn create_mine(
        ctx: Context<CreateMine>,
        _nonce_mine_together: u8,
        _nonce_mine: u8,
        fee: u64,
    ) -> ProgramResult {
        if !(fee >= MIN_MINE_FEE && fee <= MAX_MINE_FEE) {
            return Err(ErrorCode::InvalidMineFee.into());
        }

        let mine_together = &mut ctx.accounts.mine_together_account;
        let mine = &mut ctx.accounts.mine_account;
        let owner = &ctx.accounts.owner;

        // update the mine
        mine.index = mine_together.mine_counter;
        mine.owner = *owner.key;
        mine.fee = fee;

        // update the mine together
        mine_together.mine_counter += 1;

        Ok(())
    }

    #[access_control(ctx.accounts.mine_account.assert_owner(&ctx.accounts.owner))]
    pub fn update_mine_fee(
        ctx: Context<UpdateMineFee>,
        _mine_index: u32,
        _nonce_mine: u8,
        fee: u64,
    ) -> ProgramResult {
        if !(fee >= MIN_MINE_FEE && fee <= MAX_MINE_FEE) {
            return Err(ErrorCode::InvalidMineFee.into());
        }

        let mine = &mut ctx.accounts.mine_account;

        // update the mine
        mine.fee = fee;

        Ok(())
    }

    pub fn add_miners_to_mine<'a, 'b, 'c, 'info>(
        ctx: Context<'a, 'b, 'c, 'info, AddMinersToMine<'info>>,
        mine_index: u32,
        _nonce_mine: u8,
    ) -> ProgramResult {
        let mine = &mut ctx.accounts.mine_account;
        let owner = &ctx.accounts.owner;
        let remaining_accounts = ctx.remaining_accounts;
        let remaining_accounts_length = ctx.remaining_accounts.len();

        let mut index = 0;
        while index < remaining_accounts_length {
            let miner = &mut Account::<'_, MinerAccount>::try_from(&remaining_accounts[index])?;
            miner.assert_owner(owner)?;
            miner.assert_mining()?;

            if mine.total_amount == 0 || mine.x_total_amount == 0 {
                mine.x_total_amount += miner.cost;
                miner.x_aury_amount += miner.cost;
            } else {
                let what: u64 = (miner.cost as u128)
                    .checked_mul(mine.x_total_amount as u128)
                    .unwrap()
                    .checked_div(mine.total_amount as u128)
                    .unwrap()
                    .try_into()
                    .unwrap();

                mine.x_total_amount += what;
                miner.x_aury_amount += what;
            }
            mine.total_amount += miner.cost;
            miner.mine_index = mine_index;
            miner.mining_start_at = Clock::get().unwrap().unix_timestamp as u64;

            index += 1;
        }

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
#[instruction(_nonce_mine_together: u8, _miner_index: u32, _nonce_miner: u8)]
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
        seeds = [ _miner_index.to_string().as_ref(), constants::MINER_PDA_SEED.as_ref() ],
        bump = _nonce_miner,
        constraint = miner_account.owner != Pubkey::default() @ ErrorCode::MinerPurhcased,
    )]
    pub miner_account: Box<Account<'info, MinerAccount>>,

    pub admin: Signer<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(_nonce_mine_together: u8, _miner_index: u32, _nonce_miner: u8, _nonce_aury_vault: u8)]
pub struct PurchaseMiner<'info> {
    #[account(
        seeds = [ constants::MINE_TOGETHER_PDA_SEED.as_ref() ],
        bump = _nonce_mine_together,
        constraint = !mine_together_account.freeze_program @ ErrorCode::ProgramFreezed
    )]
    pub mine_together_account: Box<Account<'info, MineTogetherAccount>>,

    #[account(
        mut,
        seeds = [ _miner_index.to_string().as_ref(), constants::MINER_PDA_SEED.as_ref() ],
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

#[derive(Accounts)]
#[instruction(_nonce_mine_together: u8, _nonce_mine: u8)]
pub struct CreateMine<'info> {
    #[account(
        mut,
        seeds = [ constants::MINE_TOGETHER_PDA_SEED.as_ref() ],
        bump = _nonce_mine_together,
        constraint = !mine_together_account.freeze_program @ ErrorCode::ProgramFreezed
    )]
    pub mine_together_account: Box<Account<'info, MineTogetherAccount>>,

    #[account(
        init,
        payer = owner,
        seeds = [ mine_together_account.mine_counter.to_string().as_ref(), constants::MINE_PDA_SEED.as_ref() ],
        bump = _nonce_mine,
    )]
    pub mine_account: Box<Account<'info, MineAccount>>,

    pub owner: Signer<'info>,

    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
#[instruction(_mine_index: u32, _nonce_mine: u8)]
pub struct UpdateMineFee<'info> {
    #[account(
        mut,
        seeds = [ _mine_index.to_string().as_ref(), constants::MINE_PDA_SEED.as_ref() ],
        bump = _nonce_mine,
    )]
    pub mine_account: Box<Account<'info, MineAccount>>,

    pub owner: Signer<'info>,
}

#[derive(Accounts)]
#[instruction(mine_index: u32, _nonce_mine: u8)]
pub struct AddMinersToMine<'info> {
    #[account(
        mut,
        seeds = [ mine_index.to_string().as_ref(), constants::MINE_PDA_SEED.as_ref() ],
        bump = _nonce_mine,
    )]
    pub mine_account: Box<Account<'info, MineAccount>>,

    pub owner: Signer<'info>,
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
    pub owner: Pubkey,
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
    pub mine_index: u32,
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

impl MineAccount {
    pub fn assert_owner(&self, signer: &Signer) -> ProgramResult {
        if self.owner != *signer.key {
            return Err(ErrorCode::NotMineOwner.into());
        }

        Ok(())
    }
}

impl MinerAccount {
    pub fn assert_owner(&self, signer: &Signer) -> ProgramResult {
        if self.owner != *signer.key {
            return Err(ErrorCode::NotMinerOwner.into());
        }

        Ok(())
    }

    pub fn assert_mining(&self) -> ProgramResult {
        if self.mining_start_at > 0 {
            return Err(ErrorCode::MinerMiningStarted.into());
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
    #[msg("Invalid mine fee")]
    InvalidMineFee, // 6004, 0x1774
    #[msg("Not mine owner")]
    NotMineOwner, // 6005, 0x1775
    #[msg("Not miner owner")]
    NotMinerOwner, // 6006, 0x1776
    #[msg("Miner mining started")]
    MinerMiningStarted, // 6007, 0x1777
}
