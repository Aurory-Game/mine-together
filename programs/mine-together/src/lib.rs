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

const SHARES_LIMIT: usize = 400;

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
        name: String,
        cost: u64,
        duration: u64,
        limit: u64,
    ) -> ProgramResult {
        let mine_together = &mut ctx.accounts.mine_together_account;
        let miner = &mut ctx.accounts.miner_account;

        // update the miner
        miner.index = mine_together.miner_counter;
        miner.name = name;
        miner.cost = cost;
        miner.duration = duration;
        miner.limit = limit;

        // update the mine together
        mine_together.miner_counter += 1;

        Ok(())
    }

    #[access_control(ctx.accounts.mine_together_account.assert_admin(&ctx.accounts.admin))]
    pub fn toggle_freeze_miner(
        ctx: Context<FreezeMiner>,
        _nonce_mine_together: u8,
        _miner_index: u32,
        _nonce_miner: u8,
    ) -> ProgramResult {
        ctx.accounts.miner_account.frozen_sells = !ctx.accounts.miner_account.frozen_sells;
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
        _nonce_user_miner: u8,
        _nonce_aury_vault: u8,
        amount: u64,
    ) -> ProgramResult {
        let miner = &mut ctx.accounts.miner_account;
        let user_miner = &mut ctx.accounts.user_miner_account;
        let aury_vault = &mut ctx.accounts.aury_vault;
        let aury_from = &mut ctx.accounts.aury_from;
        let aury_from_authority = &ctx.accounts.aury_from_authority;
        let token_program = &ctx.accounts.token_program;

        miner.assert_purchasable(amount)?;

        // transfer aury to the vault
        let power = miner.cost * amount;

        spl_token_transfer(TokenTransferParams {
            source: aury_from.to_account_info(),
            destination: aury_vault.to_account_info(),
            amount: power,
            authority: aury_from_authority.to_account_info(),
            authority_signer_seeds: &[],
            token_program: token_program.to_account_info(),
        })?;

        // update the user miner
        user_miner.owner = *aury_from_authority.key;
        user_miner.miner_type = miner.key();
        user_miner.power = power;
        user_miner.duration = miner.duration;

        // update the miner
        miner.total_purchased += amount;

        Ok(())
    }

    pub fn create_mine(
        ctx: Context<CreateMine>,
        _nonce_mine_together: u8,
        _nonce_mine: u8,
        name: String,
        fee: u64,
    ) -> ProgramResult {
        if !(fee >= MIN_MINE_FEE && fee <= MAX_MINE_FEE) {
            return Err(ErrorCode::InvalidMineFee.into());
        }

        let mine = &mut ctx.accounts.mine_account;
        let fee_to = &ctx.accounts.fee_to;
        let owner = &ctx.accounts.owner;

        // update the mine
        mine.owner = *owner.key;
        mine.name = name;
        mine.fee = fee;
        mine.fee_to = fee_to.key();

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

    #[access_control(ctx.accounts.mine_account.assert_owner(&ctx.accounts.owner))]
    pub fn update_mine_fee_to(
        ctx: Context<UpdateMineFeeTo>,
        _mine_index: u32,
        _nonce_mine: u8,
    ) -> ProgramResult {
        let mine = &mut ctx.accounts.mine_account;
        let fee_to = &ctx.accounts.fee_to;

        // update the mine
        mine.fee_to = fee_to.key();

        Ok(())
    }

    #[access_control(ctx.accounts.mine_together_account.assert_admin(&ctx.accounts.admin))]
    pub fn reward_to_mine(
        ctx: Context<RewardToMine>,
        _nonce_mine_together: u8,
        _mine_index: u32,
        _nonce_mine: u8,
        _nonce_aury_vault: u8,
        amount: u64,
    ) -> ProgramResult {
        let mine = &mut ctx.accounts.mine_account;
        let aury_vault = &mut ctx.accounts.aury_vault;
        let aury_from = &mut ctx.accounts.aury_from;
        let admin = &ctx.accounts.admin;
        let token_program = &ctx.accounts.token_program;

        // transfer aury to the vault
        spl_token_transfer(TokenTransferParams {
            source: aury_from.to_account_info(),
            destination: aury_vault.to_account_info(),
            amount: amount,
            authority: admin.to_account_info(),
            authority_signer_seeds: &[],
            token_program: token_program.to_account_info(),
        })?;

        // update mine info
        mine.total_amount += amount;

        // update mine shares
        if mine.shares.len() == SHARES_LIMIT {
            mine.shares.remove(0);
        }
        let aury_share = AuryShare {
            timestamp: Clock::get().unwrap().unix_timestamp as u64,
            token_amount: mine.total_amount,
            x_token_amount: mine.x_total_amount,
        };
        mine.shares.push(aury_share);

        Ok(())
    }

    pub fn add_miners_to_mine(
        ctx: Context<AddMinersToMine>,
        mine_index: u32,
        _nonce_mine: u8,
        _miner_index: u32,
        _nonce_user_miner: u8,
    ) -> ProgramResult {
        let mine = &mut ctx.accounts.mine_account;
        let user_miner = &mut ctx.accounts.user_miner_account;
        let now = Clock::get().unwrap().unix_timestamp as u64;

        if mine.total_amount == 0 || mine.x_total_amount == 0 {
            mine.x_total_amount += user_miner.power;
            user_miner.x_aury_amount += user_miner.power;
        } else {
            let what: u64 = (user_miner.power as u128)
                .checked_mul(mine.x_total_amount as u128)
                .unwrap()
                .checked_div(mine.total_amount as u128)
                .unwrap()
                .try_into()
                .unwrap();

            mine.x_total_amount += what;
            user_miner.x_aury_amount += what;
        }
        mine.power += user_miner.power;
        mine.total_amount += user_miner.power;
        user_miner.mine_index = mine_index;
        user_miner.mining_start_at = now;

        Ok(())
    }

    pub fn claim_miner(
        ctx: Context<ClaimMiner>,
        mine_index: u32,
        _nonce_mine: u8,
        _miner_index: u32,
        _nonce_user_miner: u8,
        nonce_aury_vault: u8,
    ) -> ProgramResult {
        let mine = &mut ctx.accounts.mine_account;
        let user_miner = &mut ctx.accounts.user_miner_account;
        let aury_vault = &mut ctx.accounts.aury_vault;
        let aury_to = &mut ctx.accounts.aury_to;
        let fee_to = &mut ctx.accounts.fee_to;
        let token_program = &ctx.accounts.token_program;

        user_miner.assert_claimable(mine_index)?;

        // determine user reward amount
        let x_aury = user_miner.x_aury_amount;
        let mut what = user_miner.power;
        let mining_end_timestamp = user_miner.mining_start_at + user_miner.duration;

        for share in mine.shares.iter().rev() {
            if share.timestamp <= mining_end_timestamp {
                what = (x_aury as u128)
                    .checked_mul(share.token_amount as u128)
                    .unwrap()
                    .checked_div(share.x_token_amount as u128)
                    .unwrap()
                    .try_into()
                    .unwrap();
                break;
            }
        }

        // compute aury vault account signer seeds
        let aury_mint_key = ctx.accounts.aury_mint.key();
        let aury_vault_account_seeds = &[aury_mint_key.as_ref(), &[nonce_aury_vault]];
        let aury_vault_account_signer = &aury_vault_account_seeds[..];

        // transfer aury to the user
        let reward_amount = (what as u128)
            .checked_mul((FEE_MULTIPLIER - mine.fee) as u128)
            .unwrap()
            .checked_div(FEE_MULTIPLIER as u128)
            .unwrap()
            .try_into()
            .unwrap();
        spl_token_transfer(TokenTransferParams {
            source: aury_vault.to_account_info(),
            destination: aury_to.to_account_info(),
            amount: reward_amount,
            authority: aury_vault.to_account_info(),
            authority_signer_seeds: aury_vault_account_signer,
            token_program: token_program.to_account_info(),
        })?;

        // transfer aury fee
        let fee_amount = what - reward_amount;
        if fee_amount > 0 {
            spl_token_transfer(TokenTransferParams {
                source: aury_vault.to_account_info(),
                destination: fee_to.to_account_info(),
                amount: fee_amount,
                authority: aury_vault.to_account_info(),
                authority_signer_seeds: aury_vault_account_signer,
                token_program: token_program.to_account_info(),
            })?;
        }

        // update mine
        mine.total_amount -= what;
        mine.x_total_amount -= x_aury;
        mine.power -= user_miner.power;

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
        // 8: account's signature
        // 4: index
        // 4: name len
        // 1 * 50: name max-len 50
        // 8: cost
        // 8: duration
        // 8: limit
        // 8: total_purchased
        // 1: frozen_sells
        space = 8 + 4 + (4 + 50) + 8 + 8 + 8 + 8 + 1,
    )]
    pub miner_account: Box<Account<'info, MinerAccount>>,

    pub admin: Signer<'info>,

    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
#[instruction(_nonce_mine_together: u8, _miner_index: u32, _nonce_miner: u8)]
pub struct FreezeMiner<'info> {
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
    )]
    pub miner_account: Box<Account<'info, MinerAccount>>,

    pub admin: Signer<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(_nonce_mine_together: u8, _miner_index: u32, _nonce_miner: u8)]
pub struct RemoveMiner<'info> {
    #[account(
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
    )]
    pub miner_account: Box<Account<'info, MinerAccount>>,

    pub admin: Signer<'info>,
}

#[derive(Accounts)]
#[instruction(_nonce_mine_together: u8, _miner_index: u32, _nonce_miner: u8, _nonce_user_miner: u8, _nonce_aury_vault: u8)]
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
    )]
    pub miner_account: Box<Account<'info, MinerAccount>>,

    #[account(
        init,
        payer = aury_from_authority,
        seeds = [ _miner_index.to_string().as_ref(), constants::MINER_PDA_SEED.as_ref(), aury_from_authority.key().as_ref() ],
        bump = _nonce_user_miner,
    )]
    pub user_miner_account: Box<Account<'info, UserMinerAccount>>,

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

    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
#[instruction(_nonce_mine_together: u8, _nonce_mine: u8)]
pub struct CreateMine<'info> {
    #[account(
        seeds = [ constants::MINE_TOGETHER_PDA_SEED.as_ref() ],
        bump = _nonce_mine_together,
        constraint = !mine_together_account.freeze_program @ ErrorCode::ProgramFreezed
    )]
    pub mine_together_account: Box<Account<'info, MineTogetherAccount>>,

    #[account(
        init,
        payer = owner,
        seeds = [ owner.key().as_ref(), constants::MINE_PDA_SEED.as_ref() ],
        bump = _nonce_mine,
        // 8: account's signature
        // 32: owner
        // 4: name len
        // 1 * 50: name max-len 50
        // 8: fee
        // 32: fee_to
        // 8: power
        // 8: total amount
        // 8: x total amount
        // 4: shares vec len
        // (8 + 8 + 8) * 400: shares limit is 400
        // 8: timestamp
        // 8: token amount
        // 8: x token amount
        space = 8 + 32 + (4 + 50) + 8 + 32 + 8 + 8 + 8 + (4 + (8 + 8 + 8) * SHARES_LIMIT),
    )]
    pub mine_account: Box<Account<'info, MineAccount>>,

    #[account(
        constraint = fee_to.mint == constants::AURY_TOKEN_MINT_PUBKEY.parse::<Pubkey>().unwrap() @ ErrorCode::InvalidFeeAccount
    )]
    pub fee_to: Box<Account<'info, TokenAccount>>,

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
#[instruction(_mine_index: u32, _nonce_mine: u8)]
pub struct UpdateMineFeeTo<'info> {
    #[account(
        mut,
        seeds = [ _mine_index.to_string().as_ref(), constants::MINE_PDA_SEED.as_ref() ],
        bump = _nonce_mine,
    )]
    pub mine_account: Box<Account<'info, MineAccount>>,

    #[account(
        constraint = fee_to.mint == constants::AURY_TOKEN_MINT_PUBKEY.parse::<Pubkey>().unwrap() @ ErrorCode::InvalidFeeAccount
    )]
    pub fee_to: Box<Account<'info, TokenAccount>>,

    pub owner: Signer<'info>,
}

#[derive(Accounts)]
#[instruction(_nonce_mine_together: u8, _mine_index: u32, _nonce_mine: u8, _nonce_aury_vault: u8)]
pub struct RewardToMine<'info> {
    #[account(
        seeds = [ constants::MINE_TOGETHER_PDA_SEED.as_ref() ],
        bump = _nonce_mine_together,
        constraint = !mine_together_account.freeze_program @ ErrorCode::ProgramFreezed
    )]
    pub mine_together_account: Box<Account<'info, MineTogetherAccount>>,

    #[account(
        mut,
        seeds = [ _mine_index.to_string().as_ref(), constants::MINE_PDA_SEED.as_ref() ],
        bump = _nonce_mine,
    )]
    pub mine_account: Box<Account<'info, MineAccount>>,

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

    pub admin: Signer<'info>,

    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
#[instruction(mine_index: u32, _nonce_mine: u8, _miner_index: u32, _nonce_user_miner: u8)]
pub struct AddMinersToMine<'info> {
    #[account(
        mut,
        seeds = [ mine_index.to_string().as_ref(), constants::MINE_PDA_SEED.as_ref() ],
        bump = _nonce_mine,
    )]
    pub mine_account: Box<Account<'info, MineAccount>>,

    #[account(
        mut,
        seeds = [ _miner_index.to_string().as_ref(), constants::MINER_PDA_SEED.as_ref(), owner.key().as_ref() ],
        bump = _nonce_user_miner,
    )]
    pub user_miner_account: Box<Account<'info, UserMinerAccount>>,

    pub owner: Signer<'info>,
}

#[derive(Accounts)]
#[instruction(mine_index: u32, _nonce_mine: u8, _miner_index: u32, _nonce_user_miner: u8, nonce_aury_vault: u8)]
pub struct ClaimMiner<'info> {
    #[account(
        mut,
        seeds = [ mine_index.to_string().as_ref(), constants::MINE_PDA_SEED.as_ref() ],
        bump = _nonce_mine,
    )]
    pub mine_account: Box<Account<'info, MineAccount>>,

    #[account(
        mut,
        close = aury_to_authority,
        seeds = [ _miner_index.to_string().as_ref(), constants::MINER_PDA_SEED.as_ref(), aury_to_authority.key().as_ref() ],
        bump = _nonce_user_miner,
    )]
    pub user_miner_account: Box<Account<'info, UserMinerAccount>>,

    #[account(
        address = constants::AURY_TOKEN_MINT_PUBKEY.parse::<Pubkey>().unwrap(),
    )]
    pub aury_mint: Box<Account<'info, Mint>>,

    #[account(
        mut,
        seeds = [ constants::AURY_TOKEN_MINT_PUBKEY.parse::<Pubkey>().unwrap().as_ref() ],
        bump = nonce_aury_vault,
    )]
    pub aury_vault: Box<Account<'info, TokenAccount>>,

    #[account(mut)]
    pub aury_to: Box<Account<'info, TokenAccount>>,

    #[account(
        mut,
        constraint = fee_to.key() == mine_account.fee_to @ ErrorCode::InvalidFeeAccount
    )]
    pub fee_to: Box<Account<'info, TokenAccount>>,

    pub aury_to_authority: Signer<'info>,

    pub token_program: Program<'info, Token>,
}

#[account]
#[derive(Default)]
pub struct MineTogetherAccount {
    pub admin_key: Pubkey,
    pub freeze_program: bool,
    pub miner_counter: u32,
}

#[derive(AnchorSerialize, AnchorDeserialize, Copy, Clone, Default)]
pub struct AuryShare {
    pub timestamp: u64,
    pub token_amount: u64,
    pub x_token_amount: u64,
}

#[account]
#[derive(Default)]
pub struct MineAccount {
    pub owner: Pubkey,
    pub name: String,
    pub fee: u64,
    pub fee_to: Pubkey,
    pub power: u64,
    pub total_amount: u64,
    pub x_total_amount: u64,
    pub shares: Vec<AuryShare>,
}

#[account]
#[derive(Default)]
pub struct MinerAccount {
    pub index: u32,
    pub name: String,
    pub cost: u64,
    pub duration: u64,
    pub limit: u64,
    pub total_purchased: u64,
    pub frozen_sells: bool,
}

#[account]
#[derive(Default)]
pub struct UserMinerAccount {
    pub owner: Pubkey,
    pub miner_type: Pubkey,
    pub power: u64,
    pub duration: u64,
    pub mining_start_at: u64,
    pub mine_index: u32,
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
    pub fn assert_purchasable(&self, amount: u64) -> ProgramResult {
        if self.frozen_sells {
            return Err(ErrorCode::MinerFrozenSells.into());
        }

        if self.limit > 0 && self.total_purchased + amount > self.limit {
            return Err(ErrorCode::MinerPurchaseLimit.into());
        }

        Ok(())
    }
}

impl UserMinerAccount {
    pub fn assert_claimable(&self, mine_index: u32) -> ProgramResult {
        let now = Clock::get().unwrap().unix_timestamp as u64;

        if !(self.mining_start_at > 0) {
            return Err(ErrorCode::ClaimUnavailable.into());
        }
        if !(now >= (self.mining_start_at + self.duration)) {
            return Err(ErrorCode::ClaimUnavailable.into());
        }
        if !(self.mine_index == mine_index) {
            return Err(ErrorCode::ClaimUnavailable.into());
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
    #[msg("Miner purchase limit")]
    MinerPurchaseLimit, // 6002, 0x1772
    #[msg("Token transfer failed")]
    TokenTransferFailed, // 6003, 0x1773
    #[msg("Invalid mine fee")]
    InvalidMineFee, // 6004, 0x1774
    #[msg("Not mine owner")]
    NotMineOwner, // 6005, 0x1775
    #[msg("Not miner owner")]
    NotMinerOwner, // 6006, 0x1776
    #[msg("Non available miners")]
    NonAvailableMiners, // 6007, 0x1777
    #[msg("Invalid accounts")]
    InvalidAccounts, // 6008, 0x1778
    #[msg("Claim unavailable")]
    ClaimUnavailable, // 6009, 0x1779
    #[msg("Invalid fee account")]
    InvalidFeeAccount, // 6010, 0x177a
    #[msg("Miner frozen sells")]
    MinerFrozenSells, // 6011, 0x177a
}
