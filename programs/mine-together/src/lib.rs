pub mod utils;

use crate::utils::*;
use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token, TokenAccount};
use std::convert::TryInto;

declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

#[cfg(not(feature = "local-testing"))]
pub mod constants {
    pub const AURY_TOKEN_MINT_PUBKEY: &str = "AURYydfxJib1ZkTir1Jn1J9ECYUtjb6rKQVmtYaixWPP";
    pub const CONFIG_PDA_SEED: &[u8] = b"MINE_TOGETHER_CONFIG";
    pub const MINE_PDA_SEED: &[u8] = b"MINE_TOGETHER_MINE";
    pub const MINER_PDA_SEED: &[u8] = b"MINE_TOGETHER_MINER";
    pub const SHARES_LIMIT: usize = 400;
}

#[cfg(feature = "local-testing")]
pub mod constants {
    pub const AURY_TOKEN_MINT_PUBKEY: &str = "teST1ieLrLdr4MJPZ7i8mgSCLQ7rTrPRjNnyFdHFaz9";
    pub const CONFIG_PDA_SEED: &[u8] = b"MINE_TOGETHER_CONFIG";
    pub const MINE_PDA_SEED: &[u8] = b"MINE_TOGETHER_MINE";
    pub const MINER_PDA_SEED: &[u8] = b"MINE_TOGETHER_MINER";
    pub const SHARES_LIMIT: usize = 400;
}

#[program]
pub mod config {
    use super::*;

    const FEE_MULTIPLIER: u64 = 10000; // 100%
    pub const MAX_MINE_FEE: u64 = 5000; // 50%
    pub const MIN_MINE_FEE: u64 = 1000; // 10%

    pub fn initialize(
        ctx: Context<Initialize>,
        _nonce_config: u8,
        _nonce_aury_vault: u8,
        mine_update_delay: u64,
    ) -> ProgramResult {
        let config_account = &mut ctx.accounts.config_account;

        config_account.admin_key = *ctx.accounts.initializer.key;
        config_account.mine_update_delay = mine_update_delay;

        Ok(())
    }

    #[access_control(ctx.accounts.config_account.assert_admin(&ctx.accounts.admin))]
    pub fn update_admin(
        ctx: Context<UpdateConfig>,
        _nonce_config: u8,
        new_admin: Pubkey,
    ) -> ProgramResult {
        let config_account = &mut ctx.accounts.config_account;

        config_account.admin_key = new_admin;

        Ok(())
    }

    #[access_control(ctx.accounts.config_account.assert_admin(&ctx.accounts.admin))]
    pub fn toggle_freeze_program(ctx: Context<UpdateConfig>, _nonce_config: u8) -> ProgramResult {
        let config_account = &mut ctx.accounts.config_account;

        config_account.freeze_program = !config_account.freeze_program;

        Ok(())
    }

    #[access_control(ctx.accounts.config_account.assert_admin(&ctx.accounts.admin))]
    pub fn update_mine_update_delay(
        ctx: Context<UpdateConfig>,
        _nonce_config: u8,
        new_mine_update_delay: u64,
    ) -> ProgramResult {
        let config_account = &mut ctx.accounts.config_account;

        config_account.mine_update_delay = new_mine_update_delay;

        Ok(())
    }

    #[access_control(ctx.accounts.config_account.assert_admin(&ctx.accounts.admin))]
    pub fn create_miner(
        ctx: Context<CreateMiner>,
        _nonce_config: u8,
        _miner_created_at: u64,
        _nonce_miner: u8,
        name: String,
        cost: u64,
        duration: u64,
        limit: u64,
    ) -> ProgramResult {
        let miner_account = &mut ctx.accounts.miner_account;

        // update the miner
        miner_account.name = name;
        miner_account.cost = cost;
        miner_account.duration = duration;
        miner_account.limit = limit;

        Ok(())
    }

    #[access_control(ctx.accounts.config_account.assert_admin(&ctx.accounts.admin))]
    pub fn toggle_freeze_miner(ctx: Context<FreezeMiner>, _nonce_config: u8) -> ProgramResult {
        let miner_account = &mut ctx.accounts.miner_account;

        miner_account.frozen_sales = !miner_account.frozen_sales;

        Ok(())
    }

    #[access_control(ctx.accounts.config_account.assert_admin(&ctx.accounts.admin))]
    pub fn remove_miner(ctx: Context<RemoveMiner>, _nonce_config: u8) -> ProgramResult {
        Ok(())
    }

    pub fn purchase_miner(
        ctx: Context<PurchaseMiner>,
        _nonce_config: u8,
        _nonce_user_miner: u8,
        _nonce_aury_vault: u8,
        amount: u64,
    ) -> ProgramResult {
        let miner_account = &mut ctx.accounts.miner_account;
        let user_miner_account = &mut ctx.accounts.user_miner_account;
        let aury_vault = &mut ctx.accounts.aury_vault;
        let aury_from = &mut ctx.accounts.aury_from;
        let aury_from_authority = &ctx.accounts.aury_from_authority;
        let token_program = &ctx.accounts.token_program;

        miner_account.assert_purchasable(amount)?;

        // transfer aury to the vault
        let power = miner_account.cost * amount;

        spl_token_transfer(TokenTransferParams {
            source: aury_from.to_account_info(),
            destination: aury_vault.to_account_info(),
            amount: power,
            authority: aury_from_authority.to_account_info(),
            authority_signer_seeds: &[],
            token_program: token_program.to_account_info(),
        })?;

        // update the user miner
        user_miner_account.owner = *aury_from_authority.key;
        user_miner_account.miner_type = miner_account.key();
        user_miner_account.power = power;
        user_miner_account.duration = miner_account.duration;

        // update the miner_account
        miner_account.total_purchased += amount;

        Ok(())
    }

    pub fn create_mine(
        ctx: Context<CreateMine>,
        _nonce_config: u8,
        _nonce_mine: u8,
        name: String,
        fee: u64,
    ) -> ProgramResult {
        if !(fee >= MIN_MINE_FEE && fee <= MAX_MINE_FEE) {
            return Err(ErrorCode::InvalidMineFee.into());
        }

        let mine_account = &mut ctx.accounts.mine_account;
        let fee_to = &ctx.accounts.fee_to;
        let owner = &ctx.accounts.owner;

        // update the mine_account
        mine_account.owner = *owner.key;
        mine_account.name = name;
        mine_account.fee = fee;
        mine_account.fee_to = fee_to.key();

        Ok(())
    }

    #[access_control(ctx.accounts.mine_account.assert_owner(&ctx.accounts.owner))]
    pub fn update_mine(
        ctx: Context<UpdateMine>,
        _nonce_config: u8,
        _nonce_mine: u8,
        owner: Pubkey,
        name: String,
        fee: u64,
    ) -> ProgramResult {
        let config_account = &ctx.accounts.config_account;
        let mine_account = &mut ctx.accounts.mine_account;
        let fee_to = &ctx.accounts.fee_to;

        mine_account.assert_updatable(config_account.mine_update_delay)?;
        if !(fee >= MIN_MINE_FEE && fee <= MAX_MINE_FEE) {
            return Err(ErrorCode::InvalidMineFee.into());
        }

        // update the mine_account
        mine_account.owner = owner;
        mine_account.name = name;
        mine_account.fee = fee;
        mine_account.fee_to = fee_to.key();
        mine_account.last_updated_at = Clock::get().unwrap().unix_timestamp as u64;

        Ok(())
    }

    #[access_control(ctx.accounts.config_account.assert_admin(&ctx.accounts.admin))]
    pub fn reward_to_mine(
        ctx: Context<RewardToMine>,
        _nonce_config: u8,
        _nonce_aury_vault: u8,
        amount: u64,
    ) -> ProgramResult {
        let mine_account = &mut ctx.accounts.mine_account;
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

        // update mine_account info
        mine_account.total_amount += amount;

        // update mine_account shares
        if mine_account.shares.len() == constants::SHARES_LIMIT {
            mine_account.shares.remove(0);
        }
        let aury_share = AuryShare {
            timestamp: Clock::get().unwrap().unix_timestamp as u64,
            token_amount: mine_account.total_amount,
            x_token_amount: mine_account.x_total_amount,
        };
        mine_account.shares.push(aury_share);

        Ok(())
    }

    pub fn add_miners_to_mine(
        ctx: Context<AddMinersToMine>,
        _nonce_user_miner: u8,
    ) -> ProgramResult {
        let mine_account = &mut ctx.accounts.mine_account;
        let user_miner_account = &mut ctx.accounts.user_miner_account;
        let now = Clock::get().unwrap().unix_timestamp as u64;

        if mine_account.total_amount == 0 || mine_account.x_total_amount == 0 {
            mine_account.x_total_amount += user_miner_account.power;
            user_miner_account.x_aury_amount += user_miner_account.power;
        } else {
            let what: u64 = (user_miner_account.power as u128)
                .checked_mul(mine_account.x_total_amount as u128)
                .unwrap()
                .checked_div(mine_account.total_amount as u128)
                .unwrap()
                .try_into()
                .unwrap();

            mine_account.x_total_amount += what;
            user_miner_account.x_aury_amount += what;
        }
        mine_account.total_amount += user_miner_account.power;
        user_miner_account.mine_key = mine_account.key();
        user_miner_account.mining_start_at = now;

        Ok(())
    }

    pub fn claim_miner(
        ctx: Context<ClaimMiner>,
        _nonce_user_miner: u8,
        nonce_aury_vault: u8,
    ) -> ProgramResult {
        let mine_account = &mut ctx.accounts.mine_account;
        let user_miner_account = &mut ctx.accounts.user_miner_account;
        let aury_vault = &mut ctx.accounts.aury_vault;
        let aury_to = &mut ctx.accounts.aury_to;
        let fee_to = &mut ctx.accounts.fee_to;
        let token_program = &ctx.accounts.token_program;

        user_miner_account.assert_claimable(mine_account.key())?;

        // determine user reward amount
        let x_aury = user_miner_account.x_aury_amount;
        let mut what = 0;
        let mining_end_timestamp = user_miner_account.mining_start_at + user_miner_account.duration;

        for share in mine_account.shares.iter().rev() {
            if share.timestamp <= mining_end_timestamp {
                what = (x_aury as u128)
                    .checked_mul(share.token_amount as u128)
                    .unwrap()
                    .checked_div(share.x_token_amount as u128)
                    .unwrap()
                    .try_into()
                    .unwrap();
                what -= user_miner_account.power;
                break;
            }
        }

        // compute aury vault account signer seeds
        let aury_mint_key = ctx.accounts.aury_mint.key();
        let aury_vault_account_seeds = &[aury_mint_key.as_ref(), &[nonce_aury_vault]];
        let aury_vault_account_signer = &aury_vault_account_seeds[..];

        // transfer aury to the user
        let reward_amount: u64 = (what as u128)
            .checked_mul((FEE_MULTIPLIER - mine_account.fee) as u128)
            .unwrap()
            .checked_div(FEE_MULTIPLIER as u128)
            .unwrap()
            .try_into()
            .unwrap();
        spl_token_transfer(TokenTransferParams {
            source: aury_vault.to_account_info(),
            destination: aury_to.to_account_info(),
            amount: user_miner_account.power + reward_amount,
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

        // update mine_account
        mine_account.total_amount -= (what + user_miner_account.power);
        mine_account.x_total_amount -= x_aury;

        Ok(())
    }
}

#[derive(Accounts)]
#[instruction(_nonce_config: u8, _nonce_aury_vault: u8)]
pub struct Initialize<'info> {
    #[account(
        init,
        payer = initializer,
        seeds = [ constants::CONFIG_PDA_SEED.as_ref() ],
        bump = _nonce_config,
    )]
    pub config_account: Box<Account<'info, ConfigAccount>>,

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
#[instruction(_nonce_config: u8)]
pub struct UpdateConfig<'info> {
    #[account(
        mut,
        seeds = [ constants::CONFIG_PDA_SEED.as_ref() ],
        bump = _nonce_config,
    )]
    pub config_account: Box<Account<'info, ConfigAccount>>,

    pub admin: Signer<'info>,
}

#[derive(Accounts)]
#[instruction(_nonce_config: u8, _miner_created_at: u64, _nonce_miner: u8)]
pub struct CreateMiner<'info> {
    #[account(
        seeds = [ constants::CONFIG_PDA_SEED.as_ref() ],
        bump = _nonce_config,
        constraint = !config_account.freeze_program @ ErrorCode::ProgramFreezed
    )]
    pub config_account: Box<Account<'info, ConfigAccount>>,

    #[account(
        init,
        payer = admin,
        seeds = [ _miner_created_at.to_string().as_ref(), constants::MINER_PDA_SEED.as_ref() ],
        bump = _nonce_miner,
        // 8: account's signature
        // 4: name len
        // 1 * 50: name max-len 50
        // 8: cost
        // 8: duration
        // 8: limit
        // 8: total_purchased
        // 1: frozen_sales
        space = 8 + (4 + 50) + 8 + 8 + 8 + 8 + 1,
    )]
    pub miner_account: Box<Account<'info, MinerAccount>>,

    pub admin: Signer<'info>,

    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
#[instruction(_nonce_config: u8)]
pub struct FreezeMiner<'info> {
    #[account(
        seeds = [ constants::CONFIG_PDA_SEED.as_ref() ],
        bump = _nonce_config,
        constraint = !config_account.freeze_program @ ErrorCode::ProgramFreezed
    )]
    pub config_account: Box<Account<'info, ConfigAccount>>,

    #[account(mut)]
    pub miner_account: Box<Account<'info, MinerAccount>>,

    pub admin: Signer<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(_nonce_config: u8)]
pub struct RemoveMiner<'info> {
    #[account(
        seeds = [ constants::CONFIG_PDA_SEED.as_ref() ],
        bump = _nonce_config,
        constraint = !config_account.freeze_program @ ErrorCode::ProgramFreezed
    )]
    pub config_account: Box<Account<'info, ConfigAccount>>,

    #[account(
        mut,
        close = admin,
    )]
    pub miner_account: Box<Account<'info, MinerAccount>>,

    pub admin: Signer<'info>,
}

#[derive(Accounts)]
#[instruction(_nonce_config: u8, _nonce_user_miner: u8, _nonce_aury_vault: u8)]
pub struct PurchaseMiner<'info> {
    #[account(
        seeds = [ constants::CONFIG_PDA_SEED.as_ref() ],
        bump = _nonce_config,
        constraint = !config_account.freeze_program @ ErrorCode::ProgramFreezed
    )]
    pub config_account: Box<Account<'info, ConfigAccount>>,

    #[account(mut)]
    pub miner_account: Box<Account<'info, MinerAccount>>,

    #[account(
        init,
        payer = aury_from_authority,
        seeds = [ miner_account.key().as_ref(), constants::MINER_PDA_SEED.as_ref(), aury_from_authority.key().as_ref() ],
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
#[instruction(_nonce_config: u8, _nonce_mine: u8)]
pub struct CreateMine<'info> {
    #[account(
        seeds = [ constants::CONFIG_PDA_SEED.as_ref() ],
        bump = _nonce_config,
        constraint = !config_account.freeze_program @ ErrorCode::ProgramFreezed
    )]
    pub config_account: Box<Account<'info, ConfigAccount>>,

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
        // 8: total amount
        // 8: x total amount
        // 8: last_updated_at
        // 4: shares vec len
        // (8 + 8 + 8) * 400: shares limit is 400
        // 8: timestamp
        // 8: token amount
        // 8: x token amount
        space = 8 + 32 + (4 + 50) + 8 + 32 + 8 + 8 + 8 + (4 + (8 + 8 + 8) * constants::SHARES_LIMIT),
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
#[instruction(_nonce_config: u8, _nonce_mine: u8)]
pub struct UpdateMine<'info> {
    #[account(
        seeds = [ constants::CONFIG_PDA_SEED.as_ref() ],
        bump = _nonce_config,
        constraint = !config_account.freeze_program @ ErrorCode::ProgramFreezed
    )]
    pub config_account: Box<Account<'info, ConfigAccount>>,

    #[account(
        mut,
        seeds = [ owner.key().as_ref(), constants::MINE_PDA_SEED.as_ref() ],
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
#[instruction(_nonce_config: u8, _nonce_aury_vault: u8)]
pub struct RewardToMine<'info> {
    #[account(
        seeds = [ constants::CONFIG_PDA_SEED.as_ref() ],
        bump = _nonce_config,
        constraint = !config_account.freeze_program @ ErrorCode::ProgramFreezed
    )]
    pub config_account: Box<Account<'info, ConfigAccount>>,

    #[account(mut)]
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
#[instruction(_nonce_user_miner: u8)]
pub struct AddMinersToMine<'info> {
    #[account(mut)]
    pub mine_account: Box<Account<'info, MineAccount>>,

    #[account(
        mut,
        seeds = [ user_miner_account.miner_type.as_ref(), constants::MINER_PDA_SEED.as_ref(), owner.key().as_ref() ],
        bump = _nonce_user_miner,
    )]
    pub user_miner_account: Box<Account<'info, UserMinerAccount>>,

    pub owner: Signer<'info>,
}

#[derive(Accounts)]
#[instruction(_nonce_user_miner: u8, nonce_aury_vault: u8)]
pub struct ClaimMiner<'info> {
    #[account(mut)]
    pub mine_account: Box<Account<'info, MineAccount>>,

    #[account(
        mut,
        close = aury_to_authority,
        seeds = [ user_miner_account.miner_type.as_ref(), constants::MINER_PDA_SEED.as_ref(), aury_to_authority.key().as_ref() ],
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
pub struct ConfigAccount {
    pub admin_key: Pubkey,
    pub freeze_program: bool,
    pub mine_update_delay: u64,
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
    pub total_amount: u64,
    pub x_total_amount: u64,
    pub last_updated_at: u64,
    pub shares: Vec<AuryShare>,
}

#[account]
#[derive(Default)]
pub struct MinerAccount {
    pub name: String,
    pub cost: u64,
    pub duration: u64,
    pub limit: u64,
    pub total_purchased: u64,
    pub frozen_sales: bool,
}

#[account]
#[derive(Default)]
pub struct UserMinerAccount {
    pub owner: Pubkey,
    pub miner_type: Pubkey,
    pub power: u64,
    pub duration: u64,
    pub mining_start_at: u64,
    pub mine_key: Pubkey,
    pub x_aury_amount: u64,
}

impl ConfigAccount {
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

    pub fn assert_updatable(&self, mine_update_delay: u64) -> ProgramResult {
        let now = Clock::get().unwrap().unix_timestamp as u64;

        if (now - self.last_updated_at) < mine_update_delay {
            return Err(ErrorCode::NotOverMineUpdateDelay.into());
        }

        Ok(())
    }
}

impl MinerAccount {
    pub fn assert_purchasable(&self, amount: u64) -> ProgramResult {
        if self.frozen_sales {
            return Err(ErrorCode::MinerFrozenSells.into());
        }

        if self.limit > 0 && self.total_purchased + amount > self.limit {
            return Err(ErrorCode::MinerPurchaseLimit.into());
        }

        Ok(())
    }
}

impl UserMinerAccount {
    pub fn assert_claimable(&self, mine_key: Pubkey) -> ProgramResult {
        let now = Clock::get().unwrap().unix_timestamp as u64;

        if !(self.mining_start_at > 0) {
            return Err(ErrorCode::ClaimUnavailable.into());
        }
        if !(now >= (self.mining_start_at + self.duration)) {
            return Err(ErrorCode::ClaimUnavailable.into());
        }
        if !(self.mine_key == mine_key) {
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
    #[msg("Not over mine update delay")]
    NotOverMineUpdateDelay, // 6012, 0x177b
}
